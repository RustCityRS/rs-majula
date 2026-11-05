use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::loc::LocTypeRaw;
use crate::cache::provider::CacheType;
use crate::pack::pack_registry::{PackFile, PackRegistry};
use crate::types::ParamValue;

const ARCHIVE_LOCS: u8 = 16;
const LOC_STRIDE: u32 = 256;

pub fn unpack_locs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_LOCS).unwrap_or(0);

    let out_path = output_dir.join("all.loc");
    let mut file = std::fs::File::create(&out_path)?;

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    for group_id in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_LOCS, group_id)? else {
            continue;
        };

        // Sort entries by id within group for deterministic output.
        let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
        sorted.sort_by_key(|e| e.id);

        for entry in sorted {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = group_id * LOC_STRIDE + entry.id;
            let mut def = LocTypeRaw::new(global_id as u16);
            def.decode(&mut Packet::from(entry.data.to_vec()));

            let base = registry
                .loc
                .get_by_id(global_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("loc_{}", global_id));
            let section = if seen.contains_key(&base) {
                format!("{base}__{}", global_id)
            } else {
                base
            };
            seen.insert(section.clone(), global_id);

            super::common::write_config_section(
                &mut file,
                &section,
                emit_loc_props(&def, registry),
            )?;
            written += 1;
        }
    }

    info!("  Wrote {written} loc configs to {}", out_path.display());
    Ok(())
}

/// Server-only keys for LOC configs. These fields exist only in the server
/// stream and are NOT present in the original Jagex cache, so a decode of the
/// cache cannot reproduce them. [`update_specific_loc_file`] keeps them
/// verbatim when it refreshes an authored file's cache fields.
///
/// Note: `op1`-`op5` exist in the client cache as opcodes 30-34 and are
/// emitted by `emit_loc_props`, so they are NOT server-only.
const LOC_SERVER_ONLY_KEYS: &[&str] = &["param", "paramint", "paramstr", "category"];

/// Update a specific `.loc` file in-place: regenerate client fields from the
/// original cache while preserving server-only additions.
pub fn update_specific_loc_file(
    cache_dir: &Path,
    registry: &PackRegistry,
    file_path: &Path,
) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    super::common::update_specific_file::<LocTypeRaw, _, _>(
        &mut store,
        file_path,
        ARCHIVE_LOCS,
        LOC_STRIDE,
        LOC_SERVER_ONLY_KEYS,
        |name| registry.loc.get_by_debugname(name),
        |def| emit_loc_props(def, registry),
    )
}

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
                .filter(|name| pack.get_by_debugname(name) == Some(id as u32))
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
