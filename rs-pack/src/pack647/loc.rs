#![allow(clippy::unnecessary_to_owned)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Loc config packer — .loc text → loc.dat/loc.idx (server-side fields)
//! + loc.client.dat/.idx (client cache opcodes).
//!
//! Server stream: reads all .loc files, encodes category/param/debugname.
//! Client stream: reads `_unpack/all.loc`, encodes ALL cache opcodes directly
//! as binary (no DefinitionEncode, no LocTypeRaw — raw p1/p2/p3/p4/pjstr).
//!
//! Loc configs are special: one `[tree]` section can map to MANY loc IDs
//! via loc.pack fan-out. The compiler emits params to EVERY matching ID.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result, bail};
use tracing::{info, warn};

use super::{collect_config_files, parse_config_sections};
use crate::cache::loc::LocTypeRaw;
use crate::pack_registry::{PackFile, PackRegistry};
use crate::packed_data::PackedData;
use crate::types::ParamValue;

pub fn pack_locs(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    // ── Server stream ─────────────────────────────────────────────────────
    let files = collect_config_files(source_dir, "loc");
    info!("  Found {} .loc files", files.len());
    let configs = parse_config_sections(&files, constants);
    info!("  Parsed {} loc configs", configs.len());

    // Build reverse map: loc_name → Vec<id> for fan-out
    let mut name_to_ids: HashMap<String, Vec<u32>> = HashMap::new();
    for id in 0..registry.loc.max {
        if let Some(name) = registry.loc.get_by_id(id) {
            name_to_ids.entry(name.to_string()).or_default().push(id);
        }
    }

    // Load param type info for type-aware resolution.
    // Parse specific .param files and _unpack/all.param separately to avoid
    // cross-file duplicate panics (a param can exist in both all.param as an
    // empty cache entry and a specific .param with server-only fields).
    let param_files = collect_config_files(source_dir, "param");
    let all_param = source_dir.join("_unpack").join("all.param");
    let specific_param_files: Vec<_> = param_files
        .into_iter()
        .filter(|p| !p.components().any(|c| c.as_os_str() == "_unpack"))
        .collect();
    let mut param_configs = if all_param.exists() {
        parse_config_sections(&[all_param], constants)
    } else {
        HashMap::new()
    };
    if !specific_param_files.is_empty() {
        for (name, props) in parse_config_sections(&specific_param_files, constants) {
            param_configs.insert(name, props);
        }
    }
    let param_types: HashMap<String, String> = param_configs
        .iter()
        .filter_map(|(name, props)| {
            props
                .iter()
                .find(|(k, _)| k == "type")
                .map(|(_, v)| (name.clone(), v.clone()))
        })
        .collect();

    // Build per-ID property sets via fan-out
    let mut per_id: HashMap<u32, Vec<(String, String)>> = HashMap::new();
    for (name, props) in &configs {
        let ids = name_to_ids.get(name).map(|v| v.as_slice()).unwrap_or(&[]);
        for &id in ids {
            per_id.insert(id, props.clone());
        }
    }

    let mut packed = PackedData::new(registry.loc.max);

    for id in 0..registry.loc.max {
        packed.start_entry();

        let Some(props) = per_id.get(&id) else {
            packed.finish_entry();
            continue;
        };
        let name = registry.loc.get_by_id(id).unwrap_or("?");

        for (key, value) in props {
            match key.as_str() {
                // The examine text. 647's client has none; the server answers
                // OPLOC6 with it.
                "desc" => {
                    packed.p1(3);
                    packed.pjstr(value);
                }
                "category" => {
                    if let Some(cat_id) = registry.category.get_by_name(value) {
                        packed.p1(61);
                        packed.p2(cat_id as u16);
                    }
                }
                "op1" | "op2" | "op3" | "op4" | "op5" => {
                    let slot = key.as_bytes()[2] - b'1';
                    packed.p1(30 + slot);
                    packed.pjstr(value);
                }
                "param" => {
                    // Handled below
                }
                _ => {
                    // Client-side fields skipped
                }
            }
        }

        // Params (opcode 249) — resolve values symbolically
        let params: Vec<(&str, &str)> = props
            .iter()
            .filter(|(k, _)| k == "param")
            .filter_map(|(_, v)| v.split_once(','))
            .collect();
        if !params.is_empty() {
            packed.p1(249);
            packed.p1(params.len() as u8);
            for (pname, pval) in &params {
                let pname = pname.trim();
                let pval = pval.trim();
                let Some(pid) = registry.param.get_by_name(pname) else {
                    warn!("Unknown param '{}' for loc [{name}]", pname);
                    packed.pbool(false);
                    packed.p3(0);
                    packed.p4(0);
                    continue;
                };
                if let Ok(int_val) = pval.parse::<i32>() {
                    packed.pbool(false);
                    packed.p3(pid);
                    packed.p4(int_val);
                } else {
                    let resolved = match param_types.get(pname).map(|s| s.as_str()) {
                        Some("loc") => name_to_ids
                            .get(&pval.to_string())
                            .and_then(|v| v.first())
                            .copied(),
                        Some("string") => None,
                        // A declared type resolves in its own namespace, as a
                        // npc's or obj's param does (a coord, an enum, a
                        // synth, a boolean); a name it lacks is -1, Jordan's.
                        // The guess below served only obj, seq and npc and
                        // wrote anything else, a coord too, as a string.
                        Some(ty) if ty != "int" => Some(super::param::param_int(
                            pval,
                            super::param::type_name_to_char(ty),
                            registry,
                        ) as u32),
                        // Int, or a cache param 289's type never reached
                        // (`next_loc_stage`, the cache's param 1, holds loc
                        // names): the name's own namespace, by guess.
                        _ => name_to_ids
                            .get(&pval.to_string())
                            .and_then(|v| v.first())
                            .copied()
                            .or_else(|| registry.obj.get_by_name(pval))
                            .or_else(|| registry.seq.get_by_name(pval))
                            .or_else(|| registry.npc.get_by_name(pval)),
                    };
                    if let Some(id) = resolved {
                        packed.pbool(false);
                        packed.p3(pid);
                        packed.p4(id as i32);
                    } else {
                        packed.pbool(true);
                        packed.p3(pid);
                        packed.pjstr(pval);
                    }
                }
            }
        }

        packed.p1(250);
        packed.pjstr(name);
        packed.finish_entry();
    }

    // ── Client stream — all cache opcodes from _unpack/all.loc ──────────────
    let client = pack_locs_client_data(source_dir, registry, constants)?;

    packed.save(server_dir, "loc")?;
    config_blob.add_packed_data("loc", client.dat, client.idx);
    Ok(())
}

/// Encode ALL cache opcodes from `_unpack/all.loc` into `loc.client.dat/.idx`.
///
/// Writes binary opcodes DIRECTLY to PackedData — no LocTypeRaw, no
/// DefinitionEncode. Each text key maps to exactly the opcode(s) the client
/// decoder (`LocType.parse`) reads.
fn pack_locs_client_data(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedData> {
    let all_files = collect_config_files(source_dir, "loc");
    if all_files.is_empty() {
        info!("  No .loc files found, skipping client loc pack");
        return Ok(PackedData::new(registry.loc.max));
    }

    let client_configs = parse_config_sections(&all_files, constants);
    info!("  Parsed {} client loc configs", client_configs.len());

    let mut client = PackedData::new(registry.loc.max);

    for id in 0..registry.loc.max {
        let Some(name) = registry.loc.get_by_id(id) else {
            client.skip_entry();
            continue;
        };

        let Some(props) = client_configs.get(name) else {
            client.skip_entry();
            continue;
        };

        client.start_entry();

        // ── Batch buffers (same groups as pack_loc_def) ─────────────────
        let mut model_buf: BTreeMap<usize, (i8, Vec<u16>)> = BTreeMap::new();
        let mut model_sd_buf: BTreeMap<usize, (i8, Vec<u16>)> = BTreeMap::new();
        let mut model_hd_buf: BTreeMap<usize, (i8, Vec<u16>)> = BTreeMap::new();
        let mut recol_s_buf: BTreeMap<usize, u16> = BTreeMap::new();
        let mut recol_d_buf: BTreeMap<usize, u16> = BTreeMap::new();
        let mut retex_s_buf: BTreeMap<usize, u16> = BTreeMap::new();
        let mut retex_d_buf: BTreeMap<usize, u16> = BTreeMap::new();
        let mut recolpalette_buf: BTreeMap<usize, i8> = BTreeMap::new();
        let mut random_anim_buf: BTreeMap<usize, (u16, u8)> = BTreeMap::new();
        let mut quest_buf: BTreeMap<usize, u16> = BTreeMap::new();
        let mut multi_var: Option<u16> = None;
        let mut multi_varp: Option<u16> = None;
        let mut multi_default: Option<u16> = None;
        let mut multi_buf: BTreeMap<usize, u16> = BTreeMap::new();
        let mut params_buf: Vec<(u32, bool, i32, String)> = Vec::new(); // (key, is_str, int_val, str_val)

        // ── Key classification helpers ──────────────────────────────────
        let is_model_key = |k: &str| {
            k.starts_with("model")
                && !k.starts_with("model_sd")
                && !k.starts_with("model_hd")
                && k[5..].chars().all(|c| c.is_ascii_digit())
                && !k[5..].is_empty()
        };
        let is_model_sd_key = |k: &str| {
            k.starts_with("model_sd")
                && k["model_sd".len()..].chars().all(|c| c.is_ascii_digit())
                && !k["model_sd".len()..].is_empty()
        };
        let is_model_hd_key = |k: &str| {
            k.starts_with("model_hd")
                && k["model_hd".len()..].chars().all(|c| c.is_ascii_digit())
                && !k["model_hd".len()..].is_empty()
        };
        let is_recol_s_key = |k: &str| {
            k.starts_with("recol")
                && !k.starts_with("recolpalette")
                && k.ends_with('s')
                && k[5..k.len() - 1].chars().all(|c| c.is_ascii_digit())
                && k.len() > 6
        };
        let is_recol_d_key = |k: &str| {
            k.starts_with("recol")
                && !k.starts_with("recolpalette")
                && k.ends_with('d')
                && k[5..k.len() - 1].chars().all(|c| c.is_ascii_digit())
                && k.len() > 6
        };
        let is_retex_s_key = |k: &str| {
            k.starts_with("retex")
                && k.ends_with('s')
                && k[5..k.len() - 1].chars().all(|c| c.is_ascii_digit())
                && k.len() > 6
        };
        let is_retex_d_key = |k: &str| {
            k.starts_with("retex")
                && k.ends_with('d')
                && k[5..k.len() - 1].chars().all(|c| c.is_ascii_digit())
                && k.len() > 6
        };
        let is_recolpalette_key = |k: &str| {
            k.starts_with("recolpalette")
                && k["recolpalette".len()..]
                    .chars()
                    .all(|c| c.is_ascii_digit())
                && !k["recolpalette".len()..].is_empty()
        };
        let is_random_anim_key = |k: &str| {
            k.starts_with("random_anim")
                && k["random_anim".len()..].chars().all(|c| c.is_ascii_digit())
                && !k["random_anim".len()..].is_empty()
        };
        let is_quest_key = |k: &str| {
            k.starts_with("quest")
                && k["quest".len()..].chars().all(|c| c.is_ascii_digit())
                && !k["quest".len()..].is_empty()
        };
        let is_multi_key = |k: &str| k == "multivar" || k == "multiloc";
        let is_param_key = |k: &str| k == "paramint" || k == "paramstr";

        // ── Flush helpers — emit binary opcodes to PackedData ───────────

        fn flush_models_bin(out: &mut PackedData, buf: &mut BTreeMap<usize, (i8, Vec<u16>)>) {
            if buf.is_empty() {
                return;
            }
            let drained: Vec<(i8, Vec<u16>)> = std::mem::take(buf).into_values().collect();
            out.p1(1); // opcode 1
            out.p1(drained.len() as u8);
            for (shape, ids) in &drained {
                out.p1(*shape as u8);
                out.p1(ids.len() as u8);
                for id in ids {
                    out.p2(*id);
                }
            }
        }

        fn flush_models_sd_hd_bin(
            out: &mut PackedData,
            sd_buf: &mut BTreeMap<usize, (i8, Vec<u16>)>,
            hd_buf: &mut BTreeMap<usize, (i8, Vec<u16>)>,
        ) {
            if sd_buf.is_empty() && hd_buf.is_empty() {
                return;
            }
            let sd: Vec<(i8, Vec<u16>)> = std::mem::take(sd_buf).into_values().collect();
            let hd: Vec<(i8, Vec<u16>)> = std::mem::take(hd_buf).into_values().collect();
            out.p1(5); // opcode 5
            out.p1(sd.len() as u8);
            for (shape, ids) in &sd {
                out.p1(*shape as u8);
                out.p1(ids.len() as u8);
                for id in ids {
                    out.p2(*id);
                }
            }
            out.p1(hd.len() as u8);
            for (shape, ids) in &hd {
                out.p1(*shape as u8);
                out.p1(ids.len() as u8);
                for id in ids {
                    out.p2(*id);
                }
            }
        }

        fn flush_recol_bin(
            out: &mut PackedData,
            s_buf: &mut BTreeMap<usize, u16>,
            d_buf: &mut BTreeMap<usize, u16>,
        ) {
            if s_buf.is_empty() && d_buf.is_empty() {
                return;
            }
            let max_idx = s_buf.keys().chain(d_buf.keys()).max().copied().unwrap_or(0);
            out.p1(40); // opcode 40
            out.p1(max_idx as u8);
            for i in 1..=max_idx {
                out.p2(s_buf.remove(&i).unwrap_or(0));
                out.p2(d_buf.remove(&i).unwrap_or(0));
            }
            s_buf.clear();
            d_buf.clear();
        }

        fn flush_retex_bin(
            out: &mut PackedData,
            s_buf: &mut BTreeMap<usize, u16>,
            d_buf: &mut BTreeMap<usize, u16>,
        ) {
            if s_buf.is_empty() && d_buf.is_empty() {
                return;
            }
            let max_idx = s_buf.keys().chain(d_buf.keys()).max().copied().unwrap_or(0);
            out.p1(41); // opcode 41
            out.p1(max_idx as u8);
            for i in 1..=max_idx {
                out.p2(s_buf.remove(&i).unwrap_or(0));
                out.p2(d_buf.remove(&i).unwrap_or(0));
            }
            s_buf.clear();
            d_buf.clear();
        }

        fn flush_recolpalette_bin(out: &mut PackedData, buf: &mut BTreeMap<usize, i8>) {
            if buf.is_empty() {
                return;
            }
            let drained: Vec<i8> = std::mem::take(buf).into_values().collect();
            out.p1(42); // opcode 42
            out.p1(drained.len() as u8);
            for b in &drained {
                out.p1(*b as u8);
            }
        }

        fn flush_random_anim_bin(out: &mut PackedData, buf: &mut BTreeMap<usize, (u16, u8)>) {
            if buf.is_empty() {
                return;
            }
            let drained: Vec<(u16, u8)> = std::mem::take(buf).into_values().collect();
            out.p1(106); // opcode 106
            out.p1(drained.len() as u8);
            for (id, w) in &drained {
                out.p2(*id);
                out.p1(*w);
            }
        }

        fn flush_quest_bin(out: &mut PackedData, buf: &mut BTreeMap<usize, u16>) {
            if buf.is_empty() {
                return;
            }
            let drained: Vec<u16> = std::mem::take(buf).into_values().collect();
            out.p1(160); // opcode 160
            out.p1(drained.len() as u8);
            for q in &drained {
                out.p2(*q);
            }
        }

        fn flush_multi_bin(
            out: &mut PackedData,
            var: &mut Option<u16>,
            varp: &mut Option<u16>,
            default: &mut Option<u16>,
            buf: &mut BTreeMap<usize, u16>,
        ) {
            if var.is_none() && varp.is_none() && default.is_none() && buf.is_empty() {
                return;
            }
            let varbit = var.take().unwrap_or(65535);
            let varp_v = varp.take().unwrap_or(65535);
            let default_v = default.take();
            let count = buf.keys().next_back().copied().unwrap_or(0);
            let opcode: u8 = if default_v.is_some() { 92 } else { 77 };
            out.p1(opcode);
            out.p2(varbit);
            out.p2(varp_v);
            if let Some(d) = default_v {
                out.p2(d);
            }
            out.p1(count as u8);
            for i in 0..=count {
                out.p2(buf.remove(&i).unwrap_or(65535));
            }
        }

        fn flush_params_bin(out: &mut PackedData, buf: &mut Vec<(u32, bool, i32, String)>) {
            if buf.is_empty() {
                return;
            }
            let drained: Vec<(u32, bool, i32, String)> = std::mem::take(buf);
            out.p1(249); // opcode 249
            out.p1(drained.len() as u8);
            for (key, is_str, int_val, str_val) in &drained {
                if *is_str {
                    out.p1(1);
                    out.p3(*key);
                    out.pjstr(str_val);
                } else {
                    out.p1(0);
                    out.p3(*key);
                    out.p4(*int_val);
                }
            }
        }

        // ── Walk props in source order, flush batches on group change ────
        for (key, value) in props {
            let k = key.as_str();

            // Flush batch buffers whose key doesn't match this line
            if !is_model_key(k) {
                flush_models_bin(&mut client, &mut model_buf);
            }
            if !is_model_sd_key(k) && !is_model_hd_key(k) {
                flush_models_sd_hd_bin(&mut client, &mut model_sd_buf, &mut model_hd_buf);
            }
            if !is_recol_s_key(k) && !is_recol_d_key(k) {
                flush_recol_bin(&mut client, &mut recol_s_buf, &mut recol_d_buf);
            }
            if !is_retex_s_key(k) && !is_retex_d_key(k) {
                flush_retex_bin(&mut client, &mut retex_s_buf, &mut retex_d_buf);
            }
            if !is_recolpalette_key(k) {
                flush_recolpalette_bin(&mut client, &mut recolpalette_buf);
            }
            if !is_random_anim_key(k) {
                flush_random_anim_bin(&mut client, &mut random_anim_buf);
            }
            if !is_quest_key(k) {
                flush_quest_bin(&mut client, &mut quest_buf);
            }
            if !is_multi_key(k) {
                flush_multi_bin(
                    &mut client,
                    &mut multi_var,
                    &mut multi_varp,
                    &mut multi_default,
                    &mut multi_buf,
                );
            }
            if !is_param_key(k) {
                flush_params_bin(&mut client, &mut params_buf);
            }

            // ── Numbered batch keys ─────────────────────────────────────
            if is_model_key(k) {
                if let Ok(idx) = k[5..].parse::<usize>() {
                    if let Some((shape_str, ids_str)) = value.split_once(',') {
                        let shape = shape_str.trim().parse::<i32>().unwrap_or(0) as i8;
                        let ids: Vec<u16> = if ids_str.is_empty() {
                            Vec::new()
                        } else {
                            ids_str
                                .split(';')
                                .map(|s| s.trim().parse::<u16>().unwrap_or(0))
                                .collect()
                        };
                        model_buf.insert(idx, (shape, ids));
                    }
                }
                continue;
            }
            if is_model_sd_key(k) {
                if let Ok(idx) = k["model_sd".len()..].parse::<usize>() {
                    if let Some((shape_str, ids_str)) = value.split_once(',') {
                        let shape = shape_str.trim().parse::<i32>().unwrap_or(0) as i8;
                        let ids: Vec<u16> = if ids_str.is_empty() {
                            Vec::new()
                        } else {
                            ids_str
                                .split(';')
                                .map(|s| s.trim().parse::<u16>().unwrap_or(0))
                                .collect()
                        };
                        model_sd_buf.insert(idx, (shape, ids));
                    }
                }
                continue;
            }
            if is_model_hd_key(k) {
                if let Ok(idx) = k["model_hd".len()..].parse::<usize>() {
                    if let Some((shape_str, ids_str)) = value.split_once(',') {
                        let shape = shape_str.trim().parse::<i32>().unwrap_or(0) as i8;
                        let ids: Vec<u16> = if ids_str.is_empty() {
                            Vec::new()
                        } else {
                            ids_str
                                .split(';')
                                .map(|s| s.trim().parse::<u16>().unwrap_or(0))
                                .collect()
                        };
                        model_hd_buf.insert(idx, (shape, ids));
                    }
                }
                continue;
            }
            if is_recol_s_key(k) {
                if let Ok(idx) = k[5..k.len() - 1].parse::<usize>() {
                    if let Ok(v) = value.parse::<u16>() {
                        recol_s_buf.insert(idx, v);
                    }
                }
                continue;
            }
            if is_recol_d_key(k) {
                if let Ok(idx) = k[5..k.len() - 1].parse::<usize>() {
                    if let Ok(v) = value.parse::<u16>() {
                        recol_d_buf.insert(idx, v);
                    }
                }
                continue;
            }
            if is_retex_s_key(k) {
                if let Ok(idx) = k[5..k.len() - 1].parse::<usize>() {
                    if let Ok(v) = value.parse::<u16>() {
                        retex_s_buf.insert(idx, v);
                    }
                }
                continue;
            }
            if is_retex_d_key(k) {
                if let Ok(idx) = k[5..k.len() - 1].parse::<usize>() {
                    if let Ok(v) = value.parse::<u16>() {
                        retex_d_buf.insert(idx, v);
                    }
                }
                continue;
            }
            if is_recolpalette_key(k) {
                if let Ok(idx) = k["recolpalette".len()..].parse::<usize>() {
                    let v = value.parse::<i32>().unwrap_or(0) as i8;
                    recolpalette_buf.insert(idx, v);
                }
                continue;
            }
            if is_random_anim_key(k) {
                if let Ok(idx) = k["random_anim".len()..].parse::<usize>() {
                    if let Some((id_s, w_s)) = value.split_once(',') {
                        let id_s = id_s.trim();
                        let id_v = registry
                            .seq
                            .get_by_name(id_s)
                            .map(|v| v as u16)
                            .or_else(|| id_s.parse::<u16>().ok())
                            .unwrap_or(0);
                        if let Ok(wv) = w_s.trim().parse::<u8>() {
                            random_anim_buf.insert(idx, (id_v, wv));
                        }
                    }
                }
                continue;
            }
            if is_quest_key(k) {
                if let Ok(idx) = k["quest".len()..].parse::<usize>() {
                    if let Ok(v) = value.parse::<u16>() {
                        quest_buf.insert(idx, v);
                    }
                }
                continue;
            }
            // ── Single-value keys → direct binary opcodes ───────────────
            match k {
                "name" => {
                    client.p1(2);
                    client.pjstr(value);
                }
                "width" => {
                    client.p1(14);
                    client.p1(value.parse::<u8>().unwrap_or(0));
                }
                "length" => {
                    client.p1(15);
                    client.p1(value.parse::<u8>().unwrap_or(0));
                }
                "blockwalk" if value == "no" => {
                    client.p1(17);
                }
                "blockrange" if value == "no" => {
                    client.p1(18);
                }
                "active" => {
                    client.p1(19);
                    client.p1(value.parse::<u8>().unwrap_or(0));
                }
                "hillskew" if value == "yes" => {
                    client.p1(21);
                }
                "sharelight" if value == "yes" => {
                    client.p1(22);
                }
                "occlude" if value == "yes" => {
                    client.p1(23);
                }
                "anim" => {
                    let v = if value == "null" {
                        65535u16
                    } else {
                        registry
                            .seq
                            .get_by_name(value)
                            .map(|i| i as u16)
                            .or_else(|| value.parse::<u16>().ok())
                            .unwrap_or(65535)
                    };
                    client.p1(24);
                    client.p2(v);
                }
                "hasalpha" if value == "yes" => {
                    client.p1(25);
                }
                "interact_type1" if value == "yes" => {
                    client.p1(27);
                }
                "wallwidth" => {
                    client.p1(28);
                    client.p1(value.parse::<u8>().unwrap_or(0));
                }
                "ambient" => {
                    client.p1(29);
                    client.p1(value.parse::<i32>().unwrap_or(0) as i8 as u8);
                }
                "contrast" => {
                    client.p1(39);
                    client.p1(value.parse::<i32>().unwrap_or(0) as i8 as u8);
                }
                "op1" | "op2" | "op3" | "op4" | "op5" if value != "hidden" => {
                    let slot = k.as_bytes()[2] - b'1';
                    client.p1(30 + slot);
                    client.pjstr(value);
                }
                "mirror" if value == "yes" => {
                    client.p1(62);
                }
                "shadow" if value == "no" => {
                    client.p1(64);
                }
                "resizex" => {
                    client.p1(65);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "resizey" => {
                    client.p1(66);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "resizez" => {
                    client.p1(67);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "forceapproach" => {
                    client.p1(69);
                    client.p1(value.parse::<u8>().unwrap_or(0));
                }
                "offsetx" => {
                    client.p1(70);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "offsety" => {
                    client.p1(71);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "offsetz" => {
                    client.p1(72);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "forcedecor" if value == "yes" => {
                    client.p1(73);
                }
                "breakroutefinding" if value == "yes" => {
                    client.p1(74);
                }
                "raiseobject" => {
                    client.p1(75);
                    client.p1(value.parse::<u8>().unwrap_or(0));
                }
                "multivar" => {
                    let varbit = registry
                        .varbit
                        .get_by_name(value)
                        .or_else(|| value.strip_prefix("varbit_")?.parse().ok());
                    let varp = registry
                        .varp
                        .get_by_name(value)
                        .or_else(|| value.strip_prefix("varp_")?.parse().ok());
                    match (varbit, varp) {
                        (Some(id), None) => multi_var = Some(u16::try_from(id)?),
                        (None, Some(id)) => multi_varp = Some(u16::try_from(id)?),
                        (Some(_), Some(_)) => bail!("{name}: ambiguous multivar '{value}'"),
                        (None, None) => bail!("{name}: unknown multivar '{value}'"),
                    }
                }
                "multiloc" => {
                    let (index, loc) = value
                        .split_once(',')
                        .with_context(|| format!("{name}: invalid multiloc '{value}'"))?;
                    let loc = loc.trim();
                    let id = if loc == "null" {
                        65535
                    } else {
                        let id = registry
                            .loc
                            .get_by_name(loc)
                            .or_else(|| loc.strip_prefix("loc_")?.parse().ok())
                            .with_context(|| format!("{name}: unknown multiloc '{loc}'"))?;
                        u16::try_from(id)?
                    };
                    if index.trim() == "default" {
                        multi_default = Some(id);
                    } else {
                        let index = index
                            .trim()
                            .parse::<u8>()
                            .with_context(|| format!("{name}: invalid multiloc index '{index}'"))?;
                        multi_buf.insert(index as usize, id);
                    }
                }
                "ambient_sfx" => {
                    if let Some((id_s, range_s)) = value.split_once(',') {
                        client.p1(78);
                        client.p2(id_s.trim().parse::<u16>().unwrap_or(0));
                        client.p1(range_s.trim().parse::<u8>().unwrap_or(0));
                    }
                }
                "ambient_sfx_random" => {
                    let parts: Vec<&str> = value.splitn(4, ',').collect();
                    if parts.len() == 4 {
                        client.p1(79);
                        client.p2(parts[0].trim().parse::<u16>().unwrap_or(0));
                        client.p2(parts[1].trim().parse::<u16>().unwrap_or(0));
                        client.p1(parts[2].trim().parse::<u8>().unwrap_or(0));
                        let ids: Vec<u16> = if parts[3].is_empty() {
                            Vec::new()
                        } else {
                            parts[3]
                                .split(';')
                                .map(|s| s.trim().parse::<u16>().unwrap_or(0))
                                .collect()
                        };
                        client.p1(ids.len() as u8);
                        for id in &ids {
                            client.p2(*id);
                        }
                    }
                }
                "hillskew_t2" => {
                    client.p1(81);
                    client.p1(value.parse::<u8>().unwrap_or(0));
                }
                "hidden" if value == "yes" => {
                    client.p1(82);
                }
                "nominimap_visible" if value == "yes" => {
                    client.p1(88);
                }
                "noblocks_npcs" if value == "yes" => {
                    client.p1(89);
                }
                "op90" if value == "yes" => {
                    client.p1(90);
                }
                "members_only" if value == "yes" => {
                    client.p1(91);
                }
                "hillskew_t3_u16" => {
                    client.p1(93);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "hillskew_t4" if value == "yes" => {
                    client.p1(94);
                }
                "hillskew_t5" => {
                    client.p1(95);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "op97" if value == "yes" => {
                    client.p1(97);
                }
                "op98" if value == "yes" => {
                    client.p1(98);
                }
                "cursor1" => {
                    if let Some((a_s, b_s)) = value.split_once(',') {
                        client.p1(99);
                        client.p1(a_s.trim().parse::<u8>().unwrap_or(0));
                        client.p2(b_s.trim().parse::<u16>().unwrap_or(0));
                    }
                }
                "cursor2" => {
                    if let Some((a_s, b_s)) = value.split_once(',') {
                        client.p1(100);
                        client.p1(a_s.trim().parse::<u8>().unwrap_or(0));
                        client.p2(b_s.trim().parse::<u16>().unwrap_or(0));
                    }
                }
                "mapfunction" => {
                    client.p1(101);
                    client.p1(value.parse::<u8>().unwrap_or(0));
                }
                "msi_id" => {
                    client.p1(102);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "occlude_off" if value == "yes" => {
                    client.p1(103);
                }
                "interaction_sides" => {
                    client.p1(104);
                    client.p1(value.parse::<u8>().unwrap_or(0));
                }
                "op105" if value == "yes" => {
                    client.p1(105);
                }
                "mapelement" => {
                    client.p1(107);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "memberop1" | "memberop2" | "memberop3" | "memberop4" | "memberop5" => {
                    let slot = k.as_bytes()["memberop".len()] - b'1';
                    client.p1(150 + slot);
                    client.pjstr(value);
                }
                "hillskew_t3_i32" => {
                    client.p1(162);
                    client.p4(value.parse::<i32>().unwrap_or(0));
                }
                "op163" => {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() == 4 {
                        client.p1(163);
                        for p in &parts {
                            client.p1(p.trim().parse::<i32>().unwrap_or(0) as i8 as u8);
                        }
                    }
                }
                "op164" => {
                    client.p1(164);
                    client.p2(value.parse::<i32>().unwrap_or(0) as i16 as u16);
                }
                "op165" => {
                    client.p1(165);
                    client.p2(value.parse::<i32>().unwrap_or(0) as i16 as u16);
                }
                "op166" => {
                    client.p1(166);
                    client.p2(value.parse::<i32>().unwrap_or(0) as i16 as u16);
                }
                "op167" => {
                    client.p1(167);
                    client.p2(value.parse::<u16>().unwrap_or(0));
                }
                "op168" if value == "yes" => {
                    client.p1(168);
                }
                "op169" if value == "yes" => {
                    client.p1(169);
                }
                "op170" => {
                    client.p1(170);
                    let v = value.parse::<i32>().unwrap_or(0);
                    emit_smart_packed(&mut client, v);
                }
                "op171" => {
                    client.p1(171);
                    let v = value.parse::<i32>().unwrap_or(0);
                    emit_smart_packed(&mut client, v);
                }
                "op173" => {
                    if let Some((a_s, b_s)) = value.split_once(',') {
                        client.p1(173);
                        client.p2(a_s.trim().parse::<u16>().unwrap_or(0));
                        client.p2(b_s.trim().parse::<u16>().unwrap_or(0));
                    }
                }
                "op177" if value == "yes" => {
                    client.p1(177);
                }
                "op178" => {
                    client.p1(178);
                    client.p1(value.parse::<u8>().unwrap_or(0));
                }
                "paramint" => {
                    if let Some((k_s, v_s)) = value.split_once(',') {
                        let key_str = k_s.trim();
                        let key_int = registry
                            .param
                            .get_by_name(key_str)
                            .or_else(|| key_str.parse::<u32>().ok())
                            .unwrap_or(0);
                        if let Ok(int_val) = v_s.trim().parse::<i32>() {
                            params_buf.push((key_int, false, int_val, String::new()));
                        }
                    }
                }
                "paramstr" => {
                    if let Some((k_s, v_s)) = value.split_once(',') {
                        let key_str = k_s.trim();
                        let key_int = registry
                            .param
                            .get_by_name(key_str)
                            .or_else(|| key_str.parse::<u32>().ok())
                            .unwrap_or(0);
                        params_buf.push((key_int, true, 0, v_s.to_string()));
                    }
                }
                _ => {} // unknown key, ignore
            }
        }

        // Final flush after the prop loop
        flush_models_bin(&mut client, &mut model_buf);
        flush_models_sd_hd_bin(&mut client, &mut model_sd_buf, &mut model_hd_buf);
        flush_recol_bin(&mut client, &mut recol_s_buf, &mut recol_d_buf);
        flush_retex_bin(&mut client, &mut retex_s_buf, &mut retex_d_buf);
        flush_recolpalette_bin(&mut client, &mut recolpalette_buf);
        flush_random_anim_bin(&mut client, &mut random_anim_buf);
        flush_quest_bin(&mut client, &mut quest_buf);
        flush_multi_bin(
            &mut client,
            &mut multi_var,
            &mut multi_varp,
            &mut multi_default,
            &mut multi_buf,
        );
        flush_params_bin(&mut client, &mut params_buf);

        client.finish_entry();
    }

    Ok(client)
}

/// Emit a smart-int value: 0..127 as u8, 128..32767 as u16+32768.
fn emit_smart_packed(out: &mut PackedData, value: i32) {
    if (0..128).contains(&value) {
        out.p1(value as u8);
    } else {
        out.p2((value + 32768) as u16);
    }
}

// ─── Cache-byte-perfect packer (archive 16 — locs) ───────────────────────
//
// Text format follows 2004scape's `engine/tools/pack/config/LocConfig.ts`
// where keys exist there (`name`, `width`, `length`, `blockwalk=no`,
// `blockrange=no`, `active=N`, `hillskew=yes`, `sharelight=yes`,
// `occlude=yes`, `anim=<seq_name>`, `hasalpha=yes`, `wallwidth=N`,
// `ambient=N`, `contrast=N`, `op<N>=`, `recol<N>s/d=`, `retex<N>s/d=`,
// `mirror=yes`, `shadow=no`, `resize{x,y,z}=`, `forceapproach=N`,
// `offset{x,y,z}=`, `forcedecor=yes`, `breakroutefinding=yes`,
// `raiseobject=yes`, `mapfunction=N`, `category=<name>`); rev-647-only
// opcodes use descriptive keys derived from variant names
// (`hillskew_t1/t2/t3_u16/t3_i32/t4/t5`, `hidden`, `members_only`,
// `noblocks_npcs`, `nominimap_visible`, `op90/97/98/105/163/164/...`,
// `cursor1/2`, `multivar/multiloc`, `paramint`/
// `paramstr`, etc.).
//
// Cross-references resolve through `PackRegistry`:
//   - `anim=` → registry.seq
//   - `category=` → registry.category
//   - `paramint`/`paramstr` keys → registry.param

/// Produce the (key, value) pairs for a loc config's text representation.
///
/// Fields are emitted in canonical ascending opcode order. Only fields
/// that differ from their default value are written. Batched fields
/// (models, recol, retex, recolpalette, random_anims, quests, multiloc,
/// params) emit one text line per element, numbered where an index
/// disambiguates them.
///
/// The DebugName (op 250) emission is intentionally omitted because the
/// `[section]` header carries the name; the writer lifts it from the
/// registry on unpack.
pub fn emit_loc_props(def: &LocTypeRaw, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();

    let model_code = if def.hd_models.is_some() { 5 } else { 1 };
    for (prefix, models) in [
        (
            if model_code == 5 { "model_sd" } else { "model" },
            &def.models,
        ),
        ("model_hd", &def.hd_models),
    ] {
        if let Some(models) = models {
            for (index, model) in models.iter().enumerate() {
                let ids = model
                    .models
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(";");
                p.push((
                    model_code,
                    format!("{prefix}{}", index + 1),
                    format!("{},{}", model.shape as i8, ids),
                ));
            }
        }
    }

    // Op 2: name
    if let Some(name) = &def.name {
        p.push((2, "name".into(), name.to_string()));
    }

    // Op 14: width
    if def.opcode_order.contains(&14) || def.width != 1 {
        p.push((14, "width".into(), def.width.to_string()));
    }

    // Op 15: length
    if def.opcode_order.contains(&15) || def.length != 1 {
        p.push((15, "length".into(), def.length.to_string()));
    }

    // Op 17: blockwalk=no (also implies interact_type=0)
    if !def.blockwalk {
        p.push((17, "blockwalk".into(), "no".into()));
    }

    // Op 18: blockrange=no
    if !def.blockrange {
        p.push((18, "blockrange".into(), "no".into()));
    }

    // Op 19: active (default -1 before post_decode)
    if def.opcode_order.contains(&19) || def.active.is_some() {
        p.push((19, "active".into(), def.active.unwrap_or(0).to_string()));
    }

    // Op 21: hillskew=yes (hillskew == 1)
    if def.hillskew == 1 {
        p.push((21, "hillskew".into(), "yes".into()));
    }

    // Op 22: sharelight
    if def.sharelight {
        p.push((22, "sharelight".into(), "yes".into()));
    }

    // Op 23: occlude (set true)
    if def.occlude {
        p.push((23, "occlude".into(), "yes".into()));
    }

    // Op 24: anim
    if def.opcode_order.contains(&24) || def.anim.is_some() {
        let name = registry
            .seq
            .get_by_id(def.anim.unwrap_or(u16::MAX) as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("seq_{}", def.anim.unwrap_or(u16::MAX)));
        p.push((24, "anim".into(), name));
    }

    // Op 25: hasalpha
    if def.hasalpha {
        p.push((25, "hasalpha".into(), "yes".into()));
    }

    // Op 27: interact_type=1
    if def.interact_type == 1 {
        p.push((27, "interact_type1".into(), "yes".into()));
    }

    if def.opcode_order.contains(&28) || def.wallwidth != 16 {
        p.push((28, "wallwidth".into(), def.wallwidth.to_string()));
    }

    // Op 29: ambient
    if def.opcode_order.contains(&29) || def.ambient != 0 {
        p.push((29, "ambient".into(), def.ambient.to_string()));
    }

    // Op 30..34: op1..op5
    for (slot, opt) in def.op.as_deref().unwrap_or_default().iter().enumerate() {
        if let Some(text) = opt {
            p.push((30 + slot as u8, format!("op{}", slot + 1), text.to_string()));
        }
    }

    if def.opcode_order.contains(&39) || def.contrast != 0 {
        p.push((39, "contrast".into(), def.contrast.to_string()));
    }

    // Op 40: recol
    if let (Some(src), Some(dst)) = (&def.recol_s, &def.recol_d) {
        for i in 0..src.len() {
            p.push((40, format!("recol{}s", i + 1), src[i].to_string()));
            p.push((40, format!("recol{}d", i + 1), dst[i].to_string()));
        }
    }

    // Op 41: retex
    if let (Some(src), Some(dst)) = (&def.retex_s, &def.retex_d) {
        for i in 0..src.len() {
            p.push((41, format!("retex{}s", i + 1), src[i].to_string()));
            p.push((41, format!("retex{}d", i + 1), dst[i].to_string()));
        }
    }

    // Op 42: recolpalette
    if let Some(palette) = &def.recol_palette {
        for (i, b) in palette.iter().enumerate() {
            p.push((42, format!("recolpalette{}", i + 1), b.to_string()));
        }
    }

    // Op 62: mirror
    if def.mirror {
        p.push((62, "mirror".into(), "yes".into()));
    }

    // Op 64: shadow=no (default is true)
    if !def.shadow {
        p.push((64, "shadow".into(), "no".into()));
    }

    // Op 65: resizex
    if def.opcode_order.contains(&65) || def.resizex != 128 {
        p.push((65, "resizex".into(), def.resizex.to_string()));
    }

    // Op 66: resizey
    if def.opcode_order.contains(&66) || def.resizey != 128 {
        p.push((66, "resizey".into(), def.resizey.to_string()));
    }

    // Op 67: resizez
    if def.opcode_order.contains(&67) || def.resizez != 128 {
        p.push((67, "resizez".into(), def.resizez.to_string()));
    }

    // Op 69: forceapproach
    if def.opcode_order.contains(&69) || def.forceapproach as u8 != 0 {
        p.push((
            69,
            "forceapproach".into(),
            (def.forceapproach as u8).to_string(),
        ));
    }

    if def.opcode_order.contains(&70) || def.offsetx != 0 {
        let raw = def.offsetx as u16;
        p.push((70, "offsetx".into(), raw.to_string()));
    }

    if def.opcode_order.contains(&71) || def.offsety != 0 {
        let raw = def.offsety as u16;
        p.push((71, "offsety".into(), raw.to_string()));
    }

    if def.opcode_order.contains(&72) || def.offsetz != 0 {
        let raw = def.offsetz as u16;
        p.push((72, "offsetz".into(), raw.to_string()));
    }

    // Op 73: forcedecor
    if def.forcedecor {
        p.push((73, "forcedecor".into(), "yes".into()));
    }

    // Op 74: breakroutefinding
    if def.breakroutefinding {
        p.push((74, "breakroutefinding".into(), "yes".into()));
    }

    // Op 75: raiseobject
    if def.opcode_order.contains(&75) || def.raiseobject.is_some() {
        p.push((
            75,
            "raiseobject".into(),
            def.raiseobject.unwrap_or(0).to_string(),
        ));
    }

    // Op 77/92: multiloc — all sub-fields must share ONE opcode tag
    // so reorder_by_opcode keeps them together.
    if let Some(multiloc) = &def.multiloc {
        let default_transform = *multiloc.last().unwrap_or(&-1);
        let entries = &multiloc[..multiloc.len().saturating_sub(1)];
        let op = if def.opcode_order.contains(&92) || default_transform != -1 {
            92
        } else {
            77
        };
        let name = |pack: &PackFile, prefix: &str, id: i32| {
            if id == -1 {
                return "null".to_string();
            }
            pack.get_by_id(id as u32)
                .filter(|name| pack.get_by_name(name) == Some(id as u32))
                .map(str::to_string)
                .unwrap_or_else(|| format!("{prefix}_{id}"))
        };
        if def.transform_varbit != -1 {
            p.push((
                op,
                "multivar".into(),
                name(&registry.varbit, "varbit", def.transform_varbit),
            ));
        }
        if def.transform_varp != -1 {
            p.push((
                op,
                "multivar".into(),
                name(&registry.varp, "varp", def.transform_varp),
            ));
        }
        if op == 92 {
            p.push((
                op,
                "multiloc".into(),
                format!("default,{}", name(&registry.loc, "loc", default_transform)),
            ));
        }
        for (i, t) in entries.iter().enumerate() {
            p.push((
                op,
                "multiloc".into(),
                format!("{i},{}", name(&registry.loc, "loc", *t)),
            ));
        }
    }

    // Op 78: ambient_sfx
    if def.opcode_order.contains(&78) || def.ambient_sfx_id != 0 {
        p.push((
            78,
            "ambient_sfx".into(),
            format!("{},{}", def.ambient_sfx_id, def.ambient_sfx_range),
        ));
    }

    // Op 79: ambient_sfx_random
    if let Some(ids) = &def.sfx_random_ids {
        let id_str = ids
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(";");
        p.push((
            79,
            "ambient_sfx_random".into(),
            format!(
                "{},{},{},{}",
                def.sfx_random_pri, def.sfx_random_sec, def.ambient_sfx_range, id_str
            ),
        ));
    }

    // Op 81: hillskew_t2 (hillskew == 2, param = raw_byte * 256)
    if def.hillskew == 2 {
        p.push((
            81,
            "hillskew_t2".into(),
            (def.hillskew_param / 256).to_string(),
        ));
    }

    // Op 82: hidden (hide_minimap)
    if def.hide_minimap {
        p.push((82, "hidden".into(), "yes".into()));
    }

    // Op 88: nominimap_visible (minimap_visible default true, false = emit)
    if !def.minimap_visible {
        p.push((88, "nominimap_visible".into(), "yes".into()));
    }

    // Op 89: noblocks_npcs (blocks_npcs default true, false = emit)
    if !def.blocks_npcs {
        p.push((89, "noblocks_npcs".into(), "yes".into()));
    }

    // Op 90: unknown_90
    if def.unknown_90 {
        p.push((90, "op90".into(), "yes".into()));
    }

    // Op 91: members_only
    if def.members_only {
        p.push((91, "members_only".into(), "yes".into()));
    }

    // Op 93/162: hillskew_t3 (hillskew == 3)
    // Op 93 stores g2 (u16), op 162 stores g4 (i32).
    // Disambiguate by range: if param fits in 0..65535, use t3_u16.
    if def.hillskew == 3 {
        if def.hillskew_param >= 0 && def.hillskew_param <= 65535 {
            p.push((93, "hillskew_t3_u16".into(), def.hillskew_param.to_string()));
        } else {
            p.push((
                162,
                "hillskew_t3_i32".into(),
                def.hillskew_param.to_string(),
            ));
        }
    }

    // Op 94: hillskew_t4 (hillskew == 4)
    if def.hillskew == 4 {
        p.push((94, "hillskew_t4".into(), "yes".into()));
    }

    // Op 95: hillskew_t5 (hillskew == 5, param is signed → text is raw u16)
    if def.hillskew == 5 {
        let raw = (def.hillskew_param & 0xFFFF) as u16;
        p.push((95, "hillskew_t5".into(), raw.to_string()));
    }

    // Op 97: unknown_97
    if def.unknown_97 {
        p.push((97, "op97".into(), "yes".into()));
    }

    // Op 98: unknown_98
    if def.unknown_98 {
        p.push((98, "op98".into(), "yes".into()));
    }

    // Op 99: cursor_op1
    if let Some((a, b)) = def.cursor_op1 {
        p.push((99, "cursor1".into(), format!("{},{}", a, b)));
    }

    // Op 100: cursor_op2
    if let Some((a, b)) = def.cursor_op2 {
        p.push((100, "cursor2".into(), format!("{},{}", a, b)));
    }

    // Op 101: mapfunction
    if def.opcode_order.contains(&101) || def.mapfunction.unwrap_or(0) != 0 {
        p.push((
            101,
            "mapfunction".into(),
            def.mapfunction.unwrap_or(0).to_string(),
        ));
    }

    // Op 102: msi_id (map scene icon — client-only minimap icon frame)
    if def.opcode_order.contains(&102) || def.msi_id != -1 {
        p.push((102, "msi_id".into(), def.msi_id.to_string()));
    }

    // Op 103: occlude_off — preserve when original binary had it
    if def.opcode_order.contains(&103) {
        p.push((103, "occlude_off".into(), "yes".into()));
    }

    // Op 104: interaction_side_flags
    if def.opcode_order.contains(&104) || def.interaction_side_flags != 255 {
        p.push((
            104,
            "interaction_sides".into(),
            def.interaction_side_flags.to_string(),
        ));
    }

    // Op 105: unknown_105
    if def.unknown_105 {
        p.push((105, "op105".into(), "yes".into()));
    }

    // Op 106: random_anims
    if let (Some(ids), Some(weights)) = (&def.random_anim_ids, &def.random_anim_weights) {
        for (i, (id, w)) in ids.iter().zip(weights.iter()).enumerate() {
            let name = registry
                .seq
                .get_by_id(*id as u32)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("seq_{id}"));
            p.push((
                106,
                format!("random_anim{}", i + 1),
                format!("{},{}", name, w),
            ));
        }
    }

    // Op 107: map_element
    if def.opcode_order.contains(&107) || def.map_element != -1 {
        p.push((107, "mapelement".into(), def.map_element.to_string()));
    }

    // Op 150..154: members_op1..5
    for (slot, opt) in def.members_op.iter().enumerate() {
        if let Some(text) = opt {
            p.push((
                150 + slot as u8,
                format!("memberop{}", slot + 1),
                text.to_string(),
            ));
        }
    }

    // Op 160: quests
    if let Some(quests) = &def.quest_ids {
        for (i, q) in quests.iter().enumerate() {
            p.push((160, format!("quest{}", i + 1), q.to_string()));
        }
    }

    // Op 163: op163
    if def.opcode_order.contains(&163) || def.op163 != [0; 4] {
        p.push((
            163,
            "op163".into(),
            format!(
                "{},{},{},{}",
                def.op163[0], def.op163[1], def.op163[2], def.op163[3]
            ),
        ));
    }

    // Op 164: op164
    if def.opcode_order.contains(&164) || def.op164 != 0 {
        p.push((164, "op164".into(), def.op164.to_string()));
    }

    // Op 165: op165
    if def.opcode_order.contains(&165) || def.op165 != 0 {
        p.push((165, "op165".into(), def.op165.to_string()));
    }

    // Op 166: op166
    if def.opcode_order.contains(&166) || def.op166 != 0 {
        p.push((166, "op166".into(), def.op166.to_string()));
    }

    // Op 167: op167
    if def.opcode_order.contains(&167) || def.op167 != 0 {
        p.push((167, "op167".into(), def.op167.to_string()));
    }

    // Op 168: unknown_168
    if def.unknown_168 {
        p.push((168, "op168".into(), "yes".into()));
    }

    // Op 169: unknown_169
    if def.unknown_169 {
        p.push((169, "op169".into(), "yes".into()));
    }

    // Op 170: op170
    if def.opcode_order.contains(&170) || def.op170 != 0 {
        p.push((170, "op170".into(), def.op170.to_string()));
    }

    // Op 171: op171
    if def.opcode_order.contains(&171) || def.op171 != 0 {
        p.push((171, "op171".into(), def.op171.to_string()));
    }

    // Op 173: op173
    if def.opcode_order.contains(&173) || def.op173 != (0, 0) {
        p.push((
            173,
            "op173".into(),
            format!("{},{}", def.op173.0, def.op173.1),
        ));
    }

    // Op 177: unknown_177
    if def.unknown_177 {
        p.push((177, "op177".into(), "yes".into()));
    }

    // Op 178: op178
    if def.opcode_order.contains(&178) || def.op178 != 0 {
        p.push((178, "op178".into(), def.op178.to_string()));
    }

    // Op 249: params
    if let Some(params) = &def.params {
        let raw_param_values = decode_raw_params(&def.raw_opcodes);
        let param_name = |k: i32| -> String {
            registry
                .param
                .get_by_id(k as u32)
                .map(|s| s.to_string())
                .unwrap_or_else(|| k.to_string())
        };

        if !def.param_order.is_empty() && !raw_param_values.is_empty() {
            for (i, k) in def.param_order.iter().enumerate() {
                let pname = param_name(*k);
                if let Some(raw_val) = raw_param_values.get(i) {
                    match raw_val {
                        ParamValue::Int(v) => {
                            p.push((249, "paramint".into(), format!("{pname},{v}")));
                        }
                        ParamValue::String(s) => {
                            p.push((249, "paramstr".into(), format!("{pname},{s}")));
                        }
                    }
                } else if let Some(val) = params.get(k) {
                    match val {
                        ParamValue::Int(v) => {
                            p.push((249, "paramint".into(), format!("{pname},{v}")));
                        }
                        ParamValue::String(s) => {
                            p.push((249, "paramstr".into(), format!("{pname},{s}")));
                        }
                    }
                }
            }
        } else {
            let ordered: Vec<_> = if !def.param_order.is_empty() {
                def.param_order
                    .iter()
                    .filter_map(|k| params.get(k).map(|v| (*k, v)))
                    .collect()
            } else {
                let mut sorted: Vec<_> = params.iter().map(|(k, v)| (*k, v)).collect();
                sorted.sort_by_key(|(k, _)| *k);
                sorted
            };
            for (key, val) in ordered {
                let pname = param_name(key);
                match val {
                    ParamValue::Int(i) => {
                        p.push((249, "paramint".into(), format!("{pname},{i}")));
                    }
                    ParamValue::String(s) => {
                        p.push((249, "paramstr".into(), format!("{pname},{s}")));
                    }
                }
            }
        }
    }

    super::reorder_by_opcode_with_raw_fn(&def.opcode_order, p, &def.raw_opcodes, decode_loc_raw)
}

/// Decode all param entries from the raw op249 data.
fn decode_raw_params(raw_opcodes: &[(u8, Vec<u8>)]) -> Vec<ParamValue> {
    let Some((_, data)) = raw_opcodes.iter().find(|(op, _)| *op == 249) else {
        return Vec::new();
    };
    if data.is_empty() {
        return Vec::new();
    }
    let count = data[0] as usize;
    let mut pos = 1;
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        if pos >= data.len() {
            break;
        }
        let is_string = data[pos] == 1;
        pos += 1;
        pos += 3; // skip key
        if is_string {
            let start = pos;
            while pos < data.len() && data[pos] != 0 {
                pos += 1;
            }
            let s = String::from_utf8_lossy(&data[start..pos]).to_string();
            pos += 1;
            result.push(ParamValue::String(s.into_boxed_str()));
        } else {
            if pos + 3 < data.len() {
                let v =
                    i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                result.push(ParamValue::Int(v));
            }
            pos += 4;
        }
    }
    result
}

/// Decode raw LOC opcode bytes back to a (key, value) text pair.
fn decode_loc_raw(opcode: u8, raw: &[u8]) -> Option<(String, String)> {
    let r_u16 = |pos: usize| -> u16 {
        if pos + 1 < raw.len() {
            u16::from_be_bytes([raw[pos], raw[pos + 1]])
        } else {
            0
        }
    };
    let r_i16 = |pos: usize| -> i16 {
        if pos + 1 < raw.len() {
            i16::from_be_bytes([raw[pos], raw[pos + 1]])
        } else {
            0
        }
    };
    let r_i8 = |pos: usize| -> i8 { if pos < raw.len() { raw[pos] as i8 } else { 0 } };
    let r_u8 = |pos: usize| -> u8 { if pos < raw.len() { raw[pos] } else { 0 } };
    let _r_i32 = |pos: usize| -> i32 {
        if pos + 3 < raw.len() {
            i32::from_be_bytes([raw[pos], raw[pos + 1], raw[pos + 2], raw[pos + 3]])
        } else {
            0
        }
    };

    match opcode {
        14 => Some(("width".into(), r_u8(0).to_string())),
        15 => Some(("length".into(), r_u8(0).to_string())),
        19 => Some(("active".into(), r_u8(0).to_string())),
        24 => {
            let v = r_u16(0);
            if v == 0xFFFF {
                Some(("anim".into(), "null".into()))
            } else {
                // Raw numeric id since we don't have registry access here
                Some(("anim".into(), v.to_string()))
            }
        }
        28 => Some(("wallwidth".into(), r_u8(0).to_string())),
        29 => Some(("ambient".into(), r_i8(0).to_string())),
        39 => Some(("contrast".into(), r_i8(0).to_string())),
        65 => Some(("resizex".into(), r_u16(0).to_string())),
        66 => Some(("resizey".into(), r_u16(0).to_string())),
        67 => Some(("resizez".into(), r_u16(0).to_string())),
        69 => Some(("forceapproach".into(), r_u8(0).to_string())),
        70 => Some(("offsetx".into(), r_u16(0).to_string())),
        71 => Some(("offsety".into(), r_u16(0).to_string())),
        72 => Some(("offsetz".into(), r_u16(0).to_string())),
        75 => Some(("raiseobject".into(), r_u8(0).to_string())),
        78 => Some(("ambient_sfx".into(), format!("{},{}", r_u16(0), r_u8(2)))),
        99 => Some(("cursor1".into(), format!("{},{}", r_u8(0), r_u16(1)))),
        100 => Some(("cursor2".into(), format!("{},{}", r_u8(0), r_u16(1)))),
        101 => Some(("mapfunction".into(), r_u8(0).to_string())),
        102 => Some(("msi_id".into(), r_u16(0).to_string())),
        104 => Some(("interaction_sides".into(), r_u8(0).to_string())),
        107 => Some(("mapelement".into(), r_u16(0).to_string())),
        163 => Some((
            "op163".into(),
            format!("{},{},{},{}", r_i8(0), r_i8(1), r_i8(2), r_i8(3)),
        )),
        164 => Some(("op164".into(), r_i16(0).to_string())),
        165 => Some(("op165".into(), r_i16(0).to_string())),
        166 => Some(("op166".into(), r_i16(0).to_string())),
        167 => Some(("op167".into(), r_u16(0).to_string())),
        178 => Some(("op178".into(), r_u8(0).to_string())),
        61 => Some(("category".into(), r_u16(0).to_string())),
        _ => super::decode_raw_to_text(opcode, raw),
    }
}

#[cfg(test)]
mod tests {
    use super::{emit_loc_props, pack_locs_client_data};
    use crate::cache::loc::{LocType, LocTypeRaw};
    use crate::cache::provider::CacheType;
    use crate::pack_registry::PackRegistry;
    use crate::types::MultiVar;
    use rs_io::Packet;
    use std::collections::HashMap;
    use std::{env, fs, process};

    #[test]
    fn multiloc_config_preserves_both_selectors_and_defaults() {
        let dir = env::temp_dir().join(format!("rs-majula-multiloc-{}", process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut registry = PackRegistry::load(&dir).unwrap();
        registry.loc.max = 1;
        for (pack, id, name) in [
            (&mut registry.loc, 0, "subject"),
            (&mut registry.varbit, 3, "phase_bits"),
            (&mut registry.varp, 7, "phase"),
        ] {
            pack.id_to_name.insert(id, name.into());
            pack.name_to_id.insert(name.into(), id);
        }
        registry.loc.id_to_name.insert(9, "shared".into());
        registry.loc.id_to_name.insert(11, "shared".into());
        registry.loc.name_to_id.insert("shared".into(), 11);

        for (bytes, var) in [
            (
                vec![77, 0, 3, 255, 255, 2, 0, 9, 255, 255, 255, 255, 0],
                Some(MultiVar::Varbit(3)),
            ),
            (
                vec![77, 255, 255, 0, 7, 0, 0, 9, 0],
                Some(MultiVar::Varp(7)),
            ),
            (
                vec![92, 0, 3, 0, 7, 0, 11, 2, 0, 9, 255, 255, 255, 255, 0],
                Some(MultiVar::Varbit(3)),
            ),
            (vec![92, 255, 255, 255, 255, 255, 255, 0, 255, 255, 0], None),
        ] {
            let mut def = LocTypeRaw::new(0);
            def.decode(&mut Packet::from(bytes.clone()));
            let mut text = "[subject]\n".to_string();
            for (key, value) in emit_loc_props(&def, &registry) {
                assert!(key == "multivar" || key == "multiloc");
                if value != "1,null" {
                    text.push_str(&format!("{key}={value}\n"));
                }
            }
            fs::write(dir.join("all.loc"), text).unwrap();
            let packed = pack_locs_client_data(&dir, &registry, &HashMap::new()).unwrap();
            assert_eq!(packed.dat[2..], bytes);
            let loc = LocType::from(def);
            assert_eq!(loc.multivar, var);
            let default = if bytes[0] == 92 && var.is_some() {
                Some(11)
            } else {
                None
            };
            assert_eq!(loc.multi_loc(-1), default);
            assert_eq!(loc.multi_loc(1), default);
            assert_eq!(loc.multi_loc(256), default);
        }
        fs::remove_file(dir.join("all.loc")).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}
