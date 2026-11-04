//! Obj (item) packers — three flavours sharing one 2004scape-style text
//! format:
//!
//! 1. **Server cache (legacy, untouched binary)**: [`pack_objs`] reads .obj
//!    text and writes `data/pack/server/obj.dat`/`obj.idx` via `PackedData`.
//!    Server-side fields only (`weight`, `category`, `wearpos`, `param=...`).
//!    Binary output is unchanged from prior revisions.
//! 2. **Client PackedData**: [`pack_objs`] also reads `_unpack/all.obj` and
//!    writes ALL cache opcodes directly to a second `PackedData` stream as
//!    `obj.client.dat`/`obj.client.idx`. Binary opcodes are written inline
//!    (no `ItemDefinition` / `DefinitionEncode` involved).
//! 3. **Client cache (Phase 1 rebuild)**: [`emit_obj_props`] +
//!    [`pack_obj_def`] roundtrip through `ItemDefinition` for archive 19 of
//!    the JS5 client cache. `pack_obj_def(...).encode()` produces
//!    byte-perfect cache bytes for every cache-resident opcode.
//!
//! The cache packer mirrors `inv.rs`'s gold template: walks `def.emissions`
//! in source order, one `(key, value)` per emission. Batch opcodes (recol /
//! retex pairs, palette bytes, quest ids, params) buffer their entries and
//! flush as a single emission before any non-batch key — preserving the
//! cache's "one opcode with a count prefix" shape.
//!
//! Cross-references resolve through `PackRegistry`:
//!   - models: `model`, `manwear`, `manwear2/3`, `womanwear`, `womanwear2/3`,
//!     `manhead`, `manhead2`, `womanhead`, `womanhead2` → registry.model
//!   - objs: `multistack`, `certlink`, `certtemplate`, `lentlink`,
//!     `lenttemplate`, `boughtlink`, `boughttemplate`, `count<N>` →
//!     registry.obj
//!   - params: `paramint=<name>,<int>` / `paramstr=<name>,<string>` →
//!     registry.param
//!
//! Op 250 (debugname) is server-side only; rev-647 `ItemDefinition::parse`
//! does NOT read it. The `[section]` header in pack_obj_def's text format
//! is informational only, used by the unpacker as the canonical name. The
//! cache packer never emits an op 250 byte.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::{info, warn};

use crate::cache647::{ObjType, ParamValue};

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

pub fn pack_objs(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    server_dir: &Path,
    config_blob: &mut crate::config_blob::ConfigBlob,
) -> Result<()> {
    // ── Server encoding ────────────────────────────────────────────────
    let files = collect_config_files(source_dir, "obj");
    info!("  Found {} .obj files", files.len());
    let configs = parse_config_sections(&files, constants);
    let param_types = super::param::param_types(source_dir, registry, constants);
    info!("  Parsed {} obj configs", configs.len());

    let mut server = PackedData::new(registry.obj.max);

    for id in 0..registry.obj.max {
        server.start_entry();
        let Some(name) = registry.obj.get_by_id(id) else {
            server.finish_entry();
            continue;
        };
        let Some(props) = configs.get(name) else {
            server.p1(250);
            server.pjstr(name);
            server.finish_entry();
            continue;
        };

        for (key, value) in props {
            match key.as_str() {
                "desc" => {
                    server.p1(3);
                    server.pjstr(value);
                }
                "wearpos" => {
                    if let Some(slot) = parse_wearpos(value) {
                        server.p1(13);
                        server.p1(slot);
                    }
                }
                "wearpos2" => {
                    if let Some(slot) = parse_wearpos(value) {
                        server.p1(14);
                        server.p1(slot);
                    }
                }
                "wearpos3" => {
                    if let Some(slot) = parse_wearpos(value) {
                        server.p1(27);
                        server.p1(slot);
                    }
                }
                "weight" => {
                    server.p1(75);
                    server.p2s(parse_weight(value));
                }
                "category" => {
                    if let Some(cat_id) = registry.category.get_by_name(value) {
                        server.p1(94);
                        server.p2(cat_id as u16);
                    }
                }
                "dummyitem" => {
                    let v = match value.as_str() {
                        "graphic_only" => 1u8,
                        "inv_only" => 2,
                        _ => 0,
                    };
                    if v > 0 {
                        server.p1(96);
                        server.p1(v);
                    }
                }
                "stackable" => {
                    if value == "yes" {
                        server.p1(11);
                    }
                }
                // The cache's own fields the server reads too, in the
                // cache's encoding: the name `oc_name` prints, the price
                // shops and alchemy use, members, and the note, lent and
                // bought links `ObjType`'s post-decode copies across. Without
                // them every obj loads nameless, free, non-members and
                // unnotable (npcs already carry their name this way).
                "name" => {
                    server.p1(2);
                    server.pjstr(value);
                }
                "cost" => {
                    if let Ok(v) = value.parse::<i32>() {
                        server.p1(12);
                        server.p4(v);
                    }
                }
                "members" => {
                    if value == "yes" {
                        server.p1(16);
                    }
                }
                // Server-only, Jordan's opcode: an obj trades unless its
                // config says not. 647's client has no opcode 15.
                "tradeable" => {
                    if value == "no" || value == "false" {
                        server.p1(15);
                    }
                }
                "stockmarket" => {
                    if value == "yes" {
                        server.p1(65);
                    }
                }
                "team" => {
                    if let Ok(v) = value.parse::<u8>() {
                        server.p1(115);
                        server.p1(v);
                    }
                }
                "certlink" | "certtemplate" | "lentlink" | "lenttemplate" | "boughtlink"
                | "boughttemplate" => {
                    if let Some(v) = obj_id(registry, value) {
                        server.p1(match key.as_str() {
                            "certlink" => 97,
                            "certtemplate" => 98,
                            "lentlink" => 121,
                            "lenttemplate" => 122,
                            "boughtlink" => 139,
                            _ => 140,
                        });
                        server.p2(v);
                    }
                }
                "respawnrate" => {
                    if let Ok(v) = value.parse::<u16>() {
                        server.p1(201);
                        server.p2(v);
                    }
                }
                "param" | "paramint" | "paramstr" => {
                    // Handled after all props (opcode 249)
                }
                _ => {
                    if let Some(slot_str) = key.strip_prefix("op") {
                        if let Ok(slot) = slot_str.parse::<u8>() {
                            if (1..=5).contains(&slot) {
                                server.p1(29 + slot); // 30..=34
                                server.pjstr(value);
                            }
                        }
                    } else if let Some(slot_str) = key.strip_prefix("iop") {
                        if let Ok(slot) = slot_str.parse::<u8>() {
                            if (1..=5).contains(&slot) {
                                server.p1(34 + slot); // 35..=39
                                server.pjstr(value);
                            }
                        }
                    }
                }
            }
        }

        // Params (opcode 249). Accept the human `param=` key (auto-typed) plus the cache-export
        // `paramint=`/`paramstr=` keys (type forced by the key). All three share the
        // `<param_name>,<value>` shape. See docs/engine-parity/combat/fix-obj-packer-params.md.
        let params: Vec<(&str, &str, &str)> = props
            .iter()
            .filter(|(k, _)| matches!(k.as_str(), "param" | "paramint" | "paramstr"))
            .filter_map(|(k, v)| v.split_once(',').map(|(n, val)| (k.as_str(), n, val)))
            .collect();
        if !params.is_empty() {
            server.p1(249);
            server.p1(params.len() as u8);
            for (kind, pname, pval) in &params {
                let pname = pname.trim();
                let pval = pval.trim();
                let Some(pid) = registry.param.get_by_name(pname) else {
                    warn!("Unknown param '{}' for [{name}]", pname);
                    server.pbool(false);
                    server.p3(0);
                    server.p4(0);
                    continue;
                };
                match *kind {
                    // Cache-export string param: always a string value.
                    "paramstr" => {
                        server.pbool(true);
                        server.p3(pid);
                        server.pjstr(pval);
                    }
                    // Cache-export int param: always an int value.
                    "paramint" => {
                        let int_val = pval.parse::<i32>().unwrap_or_else(|_| {
                            warn!("paramint '{pname}' value '{pval}' not an int for [{name}]");
                            0
                        });
                        server.pbool(false);
                        server.p3(pid);
                        server.p4(int_val);
                    }
                    // Human `param=`: the value takes the param's declared
                    // type. Guessing by name lookup (seq, then obj, loc, npc)
                    // packed `fishing_struct,fishing_struct_shrimps` as a
                    // string, and so a script expecting a struct got text.
                    _ => match param_types.get(&pid).copied() {
                        Some('s') => {
                            server.pbool(true);
                            server.p3(pid);
                            server.pjstr(pval);
                        }
                        Some(type_char) => {
                            server.pbool(false);
                            server.p3(pid);
                            server.p4(super::param::param_int(pval, type_char, registry));
                        }
                        None => {
                            warn!("param '{pname}' has no declared type, for [{name}]");
                            server.pbool(false);
                            server.p3(pid);
                            server.p4(pval.parse().unwrap_or(-1));
                        }
                    },
                }
            }
        }

        server.p1(250);
        server.pjstr(name);
        server.finish_entry();
    }

    // ── Client encoding (all cache opcodes from _unpack/all.obj) ─────────
    let client = pack_objs_client_data(source_dir, registry, constants)?;

    server.save(server_dir, "obj")?;
    config_blob.add_packed_data("obj", client.dat, client.idx);
    Ok(())
}

fn model_id(registry: &PackRegistry, value: &str) -> Option<u16> {
    registry
        .model
        .get_by_name(value)
        .map(|id| id as u16)
        .or_else(|| value.strip_prefix("model_").and_then(|s| s.parse().ok()))
        .or_else(|| value.parse().ok())
}

fn param_id(registry: &PackRegistry, value: &str) -> Option<u32> {
    registry
        .param
        .get_by_name(value)
        .or_else(|| value.strip_prefix("param_").and_then(|s| s.parse().ok()))
        .or_else(|| value.parse().ok())
}

fn obj_id(registry: &PackRegistry, value: &str) -> Option<u16> {
    registry
        .obj
        .get_by_name(value)
        .map(|id| id as u16)
        .or_else(|| value.strip_prefix("obj_").and_then(|s| s.parse().ok()))
        .or_else(|| value.parse().ok())
}

fn pack_objs_client_data(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedData> {
    use std::collections::BTreeMap;

    let all_files = collect_config_files(source_dir, "obj");
    if all_files.is_empty() {
        info!("  No .obj files found, skipping client obj pack");
        return Ok(PackedData::new(registry.obj.max));
    }

    let client_configs = parse_config_sections(&all_files, constants);
    info!("  Parsed {} client obj configs", client_configs.len());

    let mut client = PackedData::new(registry.obj.max);

    for id in 0..registry.obj.max {
        let Some(name) = registry.obj.get_by_id(id) else {
            client.skip_entry();
            continue;
        };
        let Some(props) = client_configs.get(name) else {
            client.skip_entry();
            continue;
        };
        client.start_entry();

        // Batch buffers for coalesced opcodes.
        let mut recol_buf: BTreeMap<usize, (Option<u16>, Option<u16>)> = BTreeMap::new();
        let mut retex_buf: BTreeMap<usize, (Option<u16>, Option<u16>)> = BTreeMap::new();
        let mut palette_buf: BTreeMap<usize, i8> = BTreeMap::new();
        let mut quest_buf: BTreeMap<usize, u16> = BTreeMap::new();
        let mut params_buf: Vec<(u32, bool, String)> = Vec::new(); // (key, is_string, value)

        // Classify a key as belonging to a batch family.
        fn is_recol_key(k: &str) -> bool {
            (k.starts_with("recol") && (k.ends_with('s') || k.ends_with('d')))
                && !k.starts_with("recolpalette")
        }
        fn is_retex_key(k: &str) -> bool {
            k.starts_with("retex") && (k.ends_with('s') || k.ends_with('d'))
        }
        fn is_palette_key(k: &str) -> bool {
            k.starts_with("recolpalette")
        }
        fn is_quest_key(k: &str) -> bool {
            k.starts_with("quest")
                && k.strip_prefix("quest")
                    .map(|s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
                    .unwrap_or(false)
        }
        fn is_param_key(k: &str) -> bool {
            k == "paramint" || k == "paramstr"
        }

        // Flush helpers write the coalesced batch opcode into the client
        // PackedData stream.
        fn flush_recol(
            client: &mut PackedData,
            buf: &mut BTreeMap<usize, (Option<u16>, Option<u16>)>,
        ) {
            if buf.is_empty() {
                return;
            }
            let max_idx = *buf.keys().max().unwrap();
            client.p1(40);
            client.p1(max_idx as u8);
            for i in 1..=max_idx {
                let (s, d) = buf.get(&i).copied().unwrap_or((None, None));
                client.p2(s.unwrap_or(0));
                client.p2(d.unwrap_or(0));
            }
            buf.clear();
        }

        fn flush_retex(
            client: &mut PackedData,
            buf: &mut BTreeMap<usize, (Option<u16>, Option<u16>)>,
        ) {
            if buf.is_empty() {
                return;
            }
            let max_idx = *buf.keys().max().unwrap();
            client.p1(41);
            client.p1(max_idx as u8);
            // Client reads dst FIRST, then src.
            for i in 1..=max_idx {
                let (d, s) = buf.get(&i).copied().unwrap_or((None, None));
                client.p2(d.unwrap_or(0));
                client.p2(s.unwrap_or(0));
            }
            buf.clear();
        }

        fn flush_palette(client: &mut PackedData, buf: &mut BTreeMap<usize, i8>) {
            if buf.is_empty() {
                return;
            }
            let max_idx = *buf.keys().max().unwrap();
            client.p1(42);
            client.p1(max_idx as u8);
            for i in 1..=max_idx {
                client.p1(buf.get(&i).copied().unwrap_or(0) as u8);
            }
            buf.clear();
        }

        fn flush_quests(client: &mut PackedData, buf: &mut BTreeMap<usize, u16>) {
            if buf.is_empty() {
                return;
            }
            let max_idx = *buf.keys().max().unwrap();
            client.p1(132);
            client.p1(max_idx as u8);
            for i in 1..=max_idx {
                client.p2(buf.get(&i).copied().unwrap_or(0));
            }
            buf.clear();
        }

        fn flush_params(client: &mut PackedData, buf: &mut Vec<(u32, bool, String)>) {
            if buf.is_empty() {
                return;
            }
            client.p1(249);
            client.p1(buf.len() as u8);
            for (key, is_string, val) in buf.iter() {
                if *is_string {
                    client.p1(1);
                    client.p3(*key);
                    client.pjstr(val);
                } else {
                    client.p1(0);
                    client.p3(*key);
                    client.p4(val.parse::<i32>().unwrap_or(0));
                }
            }
            buf.clear();
        }

        for (key, value) in props {
            let k = key.as_str();

            let ir = is_recol_key(k);
            let it = is_retex_key(k);
            let ip = is_palette_key(k);
            let iq = is_quest_key(k);
            let ipm = is_param_key(k);

            // Flush batch buffers when switching away from that family.
            if !ir {
                flush_recol(&mut client, &mut recol_buf);
            }
            if !it {
                flush_retex(&mut client, &mut retex_buf);
            }
            if !ip {
                flush_palette(&mut client, &mut palette_buf);
            }
            if !iq {
                flush_quests(&mut client, &mut quest_buf);
            }
            if !ipm {
                flush_params(&mut client, &mut params_buf);
            }

            match k {
                "model" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(1);
                        client.p2(v);
                    }
                }
                "name" => {
                    client.p1(2);
                    client.pjstr(value);
                }
                "2dzoom" => {
                    if let Ok(v) = value.parse::<u16>() {
                        client.p1(4);
                        client.p2(v);
                    }
                }
                "2dxan" => {
                    if let Ok(v) = value.parse::<u16>() {
                        client.p1(5);
                        client.p2(v);
                    }
                }
                "2dyan" => {
                    if let Ok(v) = value.parse::<u16>() {
                        client.p1(6);
                        client.p2(v);
                    }
                }
                "2dxof" => {
                    if let Ok(v) = value.parse::<i16>() {
                        client.p1(7);
                        client.p2(v as u16);
                    } else if let Ok(v) = value.parse::<u16>() {
                        client.p1(7);
                        client.p2(v);
                    }
                }
                "2dyof" => {
                    if let Ok(v) = value.parse::<i16>() {
                        client.p1(8);
                        client.p2(v as u16);
                    } else if let Ok(v) = value.parse::<u16>() {
                        client.p1(8);
                        client.p2(v);
                    }
                }
                "stackable" => {
                    if value == "yes" {
                        client.p1(11);
                    }
                }
                "cost" => {
                    if let Ok(v) = value.parse::<i32>() {
                        client.p1(12);
                        client.p4(v);
                    }
                }
                "members" => {
                    if value == "yes" {
                        client.p1(16);
                    }
                }
                "multistack" => {
                    if let Some(v) = obj_id(registry, value) {
                        client.p1(18);
                        client.p2(v);
                    }
                }
                "manwear" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(23);
                        client.p2(v);
                    }
                }
                "manwear2" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(24);
                        client.p2(v);
                    }
                }
                "womanwear" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(25);
                        client.p2(v);
                    }
                }
                "womanwear2" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(26);
                        client.p2(v);
                    }
                }
                "stockmarket" => {
                    if value == "yes" {
                        client.p1(65);
                    }
                }
                "manwear3" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(78);
                        client.p2(v);
                    }
                }
                "womanwear3" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(79);
                        client.p2(v);
                    }
                }
                "manhead" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(90);
                        client.p2(v);
                    }
                }
                "womanhead" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(91);
                        client.p2(v);
                    }
                }
                "manhead2" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(92);
                        client.p2(v);
                    }
                }
                "womanhead2" => {
                    if let Some(v) = model_id(registry, value) {
                        client.p1(93);
                        client.p2(v);
                    }
                }
                "2dzan" => {
                    if let Ok(v) = value.parse::<u16>() {
                        client.p1(95);
                        client.p2(v);
                    }
                }
                "dummyobj" => {
                    if let Ok(v) = value.parse::<u8>() {
                        client.p1(96);
                        client.p1(v);
                    }
                }
                "certlink" => {
                    if let Some(v) = obj_id(registry, value) {
                        client.p1(97);
                        client.p2(v);
                    }
                }
                "certtemplate" => {
                    if let Some(v) = obj_id(registry, value) {
                        client.p1(98);
                        client.p2(v);
                    }
                }
                "resizex" => {
                    if let Ok(v) = value.parse::<u16>() {
                        client.p1(110);
                        client.p2(v);
                    }
                }
                "resizey" => {
                    if let Ok(v) = value.parse::<u16>() {
                        client.p1(111);
                        client.p2(v);
                    }
                }
                "resizez" => {
                    if let Ok(v) = value.parse::<u16>() {
                        client.p1(112);
                        client.p2(v);
                    }
                }
                "ambient" => {
                    if let Ok(v) = value.parse::<i8>() {
                        client.p1(113);
                        client.p1(v as u8);
                    }
                }
                "contrast" => {
                    // Text format stores the raw byte (not * 5).
                    if let Ok(raw) = value.parse::<i8>() {
                        client.p1(114);
                        client.p1(raw as u8);
                    }
                }
                "team" => {
                    if let Ok(v) = value.parse::<u8>() {
                        client.p1(115);
                        client.p1(v);
                    }
                }
                "lentlink" => {
                    if let Some(v) = obj_id(registry, value) {
                        client.p1(121);
                        client.p2(v);
                    }
                }
                "lenttemplate" => {
                    if let Some(v) = obj_id(registry, value) {
                        client.p1(122);
                        client.p2(v);
                    }
                }
                "manwearoff" => {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() == 3 {
                        if let (Ok(x), Ok(y), Ok(z)) = (
                            parts[0].trim().parse::<i8>(),
                            parts[1].trim().parse::<i8>(),
                            parts[2].trim().parse::<i8>(),
                        ) {
                            client.p1(125);
                            client.p1(x as u8);
                            client.p1(y as u8);
                            client.p1(z as u8);
                        }
                    }
                }
                "womanwearoff" => {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() == 3 {
                        if let (Ok(x), Ok(y), Ok(z)) = (
                            parts[0].trim().parse::<i8>(),
                            parts[1].trim().parse::<i8>(),
                            parts[2].trim().parse::<i8>(),
                        ) {
                            client.p1(126);
                            client.p1(x as u8);
                            client.p1(y as u8);
                            client.p1(z as u8);
                        }
                    }
                }
                "cursor1" => {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() == 2 {
                        if let (Ok(a), Ok(b)) = (
                            parts[0].trim().parse::<u8>(),
                            parts[1].trim().parse::<u16>(),
                        ) {
                            client.p1(127);
                            client.p1(a);
                            client.p2(b);
                        }
                    }
                }
                "cursor2" => {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() == 2 {
                        if let (Ok(a), Ok(b)) = (
                            parts[0].trim().parse::<u8>(),
                            parts[1].trim().parse::<u16>(),
                        ) {
                            client.p1(128);
                            client.p1(a);
                            client.p2(b);
                        }
                    }
                }
                "icursor1" => {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() == 2 {
                        if let (Ok(a), Ok(b)) = (
                            parts[0].trim().parse::<u8>(),
                            parts[1].trim().parse::<u16>(),
                        ) {
                            client.p1(129);
                            client.p1(a);
                            client.p2(b);
                        }
                    }
                }
                "icursor2" => {
                    let parts: Vec<&str> = value.split(',').collect();
                    if parts.len() == 2 {
                        if let (Ok(a), Ok(b)) = (
                            parts[0].trim().parse::<u8>(),
                            parts[1].trim().parse::<u16>(),
                        ) {
                            client.p1(130);
                            client.p1(a);
                            client.p2(b);
                        }
                    }
                }
                "picksizeshift" => {
                    if let Ok(v) = value.parse::<u8>() {
                        client.p1(134);
                        client.p1(v);
                    }
                }
                "boughtlink" => {
                    if let Some(v) = obj_id(registry, value) {
                        client.p1(139);
                        client.p2(v);
                    }
                }
                "boughttemplate" => {
                    if let Some(v) = obj_id(registry, value) {
                        client.p1(140);
                        client.p2(v);
                    }
                }
                "paramint" => {
                    if let Some((k_s, v_s)) = value.split_once(',') {
                        if let Some(key_int) = param_id(registry, k_s.trim()) {
                            params_buf.push((key_int, false, v_s.trim().to_string()));
                        }
                    }
                }
                "paramstr" => {
                    if let Some((k_s, v_s)) = value.split_once(',') {
                        if let Some(key_int) = param_id(registry, k_s.trim()) {
                            params_buf.push((key_int, true, v_s.to_string()));
                        }
                    }
                }
                _ => {
                    // Slot-suffixed keys: op<N>, iop<N>, count<N>,
                    // recol/retex/palette/quest batch keys.
                    if let Some(slot_str) = k.strip_prefix("op") {
                        if let Ok(slot_one) = slot_str.parse::<u8>() {
                            if (1..=5).contains(&slot_one) {
                                client.p1(29 + slot_one); // 30..=34
                                client.pjstr(value);
                            }
                        }
                    } else if let Some(slot_str) = k.strip_prefix("iop") {
                        if let Ok(slot_one) = slot_str.parse::<u8>() {
                            if (1..=5).contains(&slot_one) {
                                client.p1(34 + slot_one); // 35..=39
                                client.pjstr(value);
                            }
                        }
                    } else if let Some(slot_str) = k.strip_prefix("count") {
                        if let Ok(slot_one) = slot_str.parse::<u8>() {
                            if (1..=10).contains(&slot_one) {
                                let parts: Vec<&str> = value.split(',').collect();
                                if parts.len() == 2 {
                                    if let (Some(obj), Ok(count)) = (
                                        obj_id(registry, parts[0].trim()),
                                        parts[1].trim().parse::<u16>(),
                                    ) {
                                        client.p1(99 + slot_one); // 100..=109
                                        client.p2(obj);
                                        client.p2(count);
                                    }
                                }
                            }
                        }
                    } else if ir {
                        // recol<N>s or recol<N>d
                        let body = &k[5..]; // after "recol"
                        let (idx_str, side) = if let Some(s) = body.strip_suffix('s') {
                            (s, 's')
                        } else if let Some(s) = body.strip_suffix('d') {
                            (s, 'd')
                        } else {
                            continue;
                        };
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if idx == 0 {
                                continue;
                            }
                            if let Ok(v) = value.trim().parse::<u16>() {
                                let slot = recol_buf.entry(idx).or_insert((None, None));
                                if side == 's' {
                                    slot.0 = Some(v);
                                } else {
                                    slot.1 = Some(v);
                                }
                            }
                        }
                    } else if it {
                        // retex<N>d or retex<N>s — buffer order is (dst, src)
                        let body = &k[5..]; // after "retex"
                        let (idx_str, side) = if let Some(s) = body.strip_suffix('d') {
                            (s, 'd')
                        } else if let Some(s) = body.strip_suffix('s') {
                            (s, 's')
                        } else {
                            continue;
                        };
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if idx == 0 {
                                continue;
                            }
                            if let Ok(v) = value.trim().parse::<u16>() {
                                let slot = retex_buf.entry(idx).or_insert((None, None));
                                if side == 'd' {
                                    slot.0 = Some(v);
                                } else {
                                    slot.1 = Some(v);
                                }
                            }
                        }
                    } else if ip {
                        let idx_str = &k["recolpalette".len()..];
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if idx == 0 {
                                continue;
                            }
                            if let Ok(v) = value.trim().parse::<i8>() {
                                palette_buf.insert(idx, v);
                            }
                        }
                    } else if iq {
                        let idx_str = &k[5..]; // after "quest"
                        if let Ok(idx) = idx_str.parse::<usize>() {
                            if idx == 0 {
                                continue;
                            }
                            if let Ok(v) = value.trim().parse::<u16>() {
                                quest_buf.insert(idx, v);
                            }
                        }
                    }
                    // Unknown keys are silently ignored.
                }
            }
        }

        // Flush any remaining batch buffers.
        flush_recol(&mut client, &mut recol_buf);
        flush_retex(&mut client, &mut retex_buf);
        flush_palette(&mut client, &mut palette_buf);
        flush_quests(&mut client, &mut quest_buf);
        flush_params(&mut client, &mut params_buf);

        client.finish_entry();
    }

    Ok(client)
}

fn parse_wearpos(value: &str) -> Option<u8> {
    match value {
        "hat" => Some(0),
        "back" => Some(1),
        "front" => Some(2),
        "righthand" => Some(3),
        "torso" => Some(4),
        "lefthand" => Some(5),
        "arms" => Some(6),
        "legs" => Some(7),
        "head" => Some(8),
        "hands" => Some(9),
        "feet" => Some(10),
        "jaw" => Some(11),
        "ring" => Some(12),
        "quiver" => Some(13),
        _ => value.parse().ok(),
    }
}

// ─── Cache-byte-perfect packer (archive 19) ──────────────────────────────
//
// Mirrors `inv.rs` template. Walk `def.emissions` in source order, one
// `(key, value)` per emission. Batch opcodes (recol/retex pairs, palette
// bytes, quest ids, params) emit one text line per element and coalesce on
// read. Per-slot opcodes (op1..5, iop1..5, count1..10) carry their slot in
// the key name so the packer reconstructs the exact opcode byte.
//
// Cross-references (model/obj/param ids) resolve through `PackRegistry`.
// Names with no registry entry fall back to `model_<id>` / `obj_<id>` /
// `param_<id>` so unknown ids still roundtrip.

/// Resolve a model id to a name via the registry. Falls back to
/// `model_<id>` so unknown ids still survive a roundtrip.
fn model_name(registry: &PackRegistry, id: u16) -> String {
    registry
        .model
        .get_by_id(id as u32)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("model_{id}"))
}

/// Resolve an obj id to a name via the registry. Falls back to `obj_<id>`
/// if the id has no name OR if the registered name has a duplicate (e.g.
/// obj.pack contains the same `amulet_of_glory_t1` for both 10360 and
/// 10719). Without this guard, the pack-side `get_by_name` would resolve
/// the duplicate to the wrong id and the round-trip would diverge.
fn obj_name(registry: &PackRegistry, id: u16) -> String {
    if let Some(name) = registry.obj.get_by_id(id as u32) {
        if registry.obj.get_by_name(name) == Some(id as u32) {
            return name.to_string();
        }
    }
    format!("obj_{id}")
}

/// Resolve a param id to a name via the registry. Falls back to the raw id
/// as a decimal string so unknown params still roundtrip.
fn param_name(registry: &PackRegistry, id: u32) -> String {
    registry
        .param
        .get_by_id(id)
        .map(|s| s.to_string())
        .unwrap_or_else(|| id.to_string())
}

/// Walk an item's emissions and produce the (key, value) pairs for the
/// text config format. Used by the unpacker to produce `all.obj`.
/// Source order is preserved; duplicate emissions become duplicate lines.
///
/// Cross-references resolve through `PackRegistry`. Unknown ids fall back
/// to `<type>_<id>` placeholders so the roundtrip still produces the
/// correct cache bytes.
pub fn emit_obj_props(def: &ObjType, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) || def.model != 0 {
        p.push((1, "model".into(), model_name(registry, def.model)));
    }
    if let Some(name) = &def.name {
        p.push((2, "name".into(), name.to_string()));
    }
    if def.opcode_order.contains(&4) || def.zoom_2d != 2000 {
        p.push((4, "2dzoom".into(), def.zoom_2d.to_string()));
    }
    if def.opcode_order.contains(&5) || def.xan_2d != 0 {
        p.push((5, "2dxan".into(), def.xan_2d.to_string()));
    }
    if def.opcode_order.contains(&6) || def.yan_2d != 0 {
        p.push((6, "2dyan".into(), def.yan_2d.to_string()));
    }
    if def.opcode_order.contains(&7) || def.xof_2d != 0 {
        p.push((7, "2dxof".into(), def.xof_2d.to_string()));
    }
    if def.opcode_order.contains(&8) || def.yof_2d != 0 {
        p.push((8, "2dyof".into(), def.yof_2d.to_string()));
    }
    if def.stackable {
        p.push((11, "stackable".into(), "yes".into()));
    }
    if def.opcode_order.contains(&12) || def.cost != 1 {
        p.push((12, "cost".into(), def.cost.to_string()));
    }
    if def.members {
        p.push((16, "members".into(), "yes".into()));
    }
    if def.opcode_order.contains(&18) || def.multistack != 0 {
        p.push((18, "multistack".into(), obj_name(registry, def.multistack)));
    }
    if def.opcode_order.contains(&23) || def.manwear != 0 {
        p.push((23, "manwear".into(), model_name(registry, def.manwear)));
    }
    if def.opcode_order.contains(&24) || def.manwear2 != 0 {
        p.push((24, "manwear2".into(), model_name(registry, def.manwear2)));
    }
    if def.opcode_order.contains(&25) || def.womanwear != 0 {
        p.push((25, "womanwear".into(), model_name(registry, def.womanwear)));
    }
    if def.opcode_order.contains(&26) || def.womenwear2 != 0 {
        p.push((
            26,
            "womanwear2".into(),
            model_name(registry, def.womenwear2),
        ));
    }
    for (slot, opt) in def.options.iter().enumerate() {
        if let Some(text) = opt {
            p.push((30 + slot as u8, format!("op{}", slot + 1), text.to_string()));
        }
    }
    for (slot, opt) in def.interface_options.iter().enumerate() {
        if let Some(text) = opt {
            p.push((
                35 + slot as u8,
                format!("iop{}", slot + 1),
                text.to_string(),
            ));
        }
    }
    if let (Some(src), Some(dst)) = (&def.recol_s, &def.recol_d) {
        for i in 0..src.len() {
            let n = i + 1;
            p.push((40, format!("recol{n}s"), src[i].to_string()));
            p.push((40, format!("recol{n}d"), dst[i].to_string()));
        }
    }
    if let (Some(dst), Some(src)) = (&def.retex_d, &def.retex_s) {
        for i in 0..dst.len() {
            let n = i + 1;
            p.push((41, format!("retex{n}d"), dst[i].to_string()));
            p.push((41, format!("retex{n}s"), src[i].to_string()));
        }
    }
    if let Some(palette) = &def.recol_palette {
        for (i, b) in palette.iter().enumerate() {
            p.push((42, format!("recolpalette{}", i + 1), b.to_string()));
        }
    }
    if def.stockmarket {
        p.push((65, "stockmarket".into(), "yes".into()));
    }
    if def.opcode_order.contains(&78) || def.manwear3 != 0 {
        p.push((78, "manwear3".into(), model_name(registry, def.manwear3)));
    }
    if def.opcode_order.contains(&79) || def.womanwear3 != 0 {
        p.push((
            79,
            "womanwear3".into(),
            model_name(registry, def.womanwear3),
        ));
    }
    if def.opcode_order.contains(&90) || def.manhead != 0 {
        p.push((90, "manhead".into(), model_name(registry, def.manhead)));
    }
    if def.opcode_order.contains(&91) || def.womanhead != 0 {
        p.push((91, "womanhead".into(), model_name(registry, def.womanhead)));
    }
    if def.opcode_order.contains(&92) || def.manhead2 != 0 {
        p.push((92, "manhead2".into(), model_name(registry, def.manhead2)));
    }
    if def.opcode_order.contains(&93) || def.womenhead2 != 0 {
        p.push((
            93,
            "womanhead2".into(),
            model_name(registry, def.womenhead2),
        ));
    }
    if def.opcode_order.contains(&95) || def.zan_2d != 0 {
        p.push((95, "2dzan".into(), def.zan_2d.to_string()));
    }
    if def.opcode_order.contains(&96) || def.dummy_obj != 0 {
        p.push((96, "dummyobj".into(), def.dummy_obj.to_string()));
    }
    if def.opcode_order.contains(&97) || def.certlink != 0 {
        p.push((97, "certlink".into(), obj_name(registry, def.certlink)));
    }
    if def.opcode_order.contains(&98) || def.certtemplate != 0 {
        p.push((
            98,
            "certtemplate".into(),
            obj_name(registry, def.certtemplate),
        ));
    }
    if let (Some(countobj), Some(countco)) = (&def.countobj, &def.countco) {
        for (slot, (obj, count)) in countobj.iter().zip(countco.iter()).enumerate() {
            let opcode = 100 + slot as u8;
            if def.opcode_order.contains(&opcode) || *obj != 0 || *count != 0 {
                p.push((
                    opcode,
                    format!("count{}", slot + 1),
                    format!("{},{}", obj_name(registry, *obj), count),
                ));
            }
        }
    }
    if def.opcode_order.contains(&110) || def.resize_x != 128 {
        p.push((110, "resizex".into(), def.resize_x.to_string()));
    }
    if def.opcode_order.contains(&111) || def.resize_y != 128 {
        p.push((111, "resizey".into(), def.resize_y.to_string()));
    }
    if def.opcode_order.contains(&112) || def.resize_z != 128 {
        p.push((112, "resizez".into(), def.resize_z.to_string()));
    }
    if def.opcode_order.contains(&113) || def.ambient != 0 {
        p.push((113, "ambient".into(), def.ambient.to_string()));
    }
    if def.opcode_order.contains(&114) || def.contrast != 0 {
        p.push((114, "contrast".into(), (def.contrast / 5).to_string()));
    }
    if def.opcode_order.contains(&115) || def.team != 0 {
        p.push((115, "team".into(), def.team.to_string()));
    }
    if def.opcode_order.contains(&121) || def.lentlink != 0 {
        p.push((121, "lentlink".into(), obj_name(registry, def.lentlink)));
    }
    if def.opcode_order.contains(&122) || def.lenttemplate != 0 {
        p.push((
            122,
            "lenttemplate".into(),
            obj_name(registry, def.lenttemplate),
        ));
    }
    if def.opcode_order.contains(&125)
        || def.manwear_xoff != 0
        || def.manwear_yoff != 0
        || def.manwear_zoff != 0
    {
        p.push((
            125,
            "manwearoff".into(),
            format!(
                "{},{},{}",
                def.manwear_xoff, def.manwear_yoff, def.manwear_zoff
            ),
        ));
    }
    if def.opcode_order.contains(&126)
        || def.womanwear_xoff != 0
        || def.womanwear_yoff != 0
        || def.womanwear_zoff != 0
    {
        p.push((
            126,
            "womanwearoff".into(),
            format!(
                "{},{},{}",
                def.womanwear_xoff, def.womanwear_yoff, def.womanwear_zoff
            ),
        ));
    }
    if let Some((a, b)) = def.cursor1 {
        p.push((127, "cursor1".into(), format!("{a},{b}")));
    }
    if let Some((a, b)) = def.cursor2 {
        p.push((128, "cursor2".into(), format!("{a},{b}")));
    }
    if let Some((a, b)) = def.icursor1 {
        p.push((129, "icursor1".into(), format!("{a},{b}")));
    }
    if let Some((a, b)) = def.icursor2 {
        p.push((130, "icursor2".into(), format!("{a},{b}")));
    }
    if let Some(quests) = &def.quests {
        for (i, q) in quests.iter().enumerate() {
            p.push((132, format!("quest{}", i + 1), q.to_string()));
        }
    }
    if def.opcode_order.contains(&134) || def.picksizeshift != 0 {
        p.push((134, "picksizeshift".into(), def.picksizeshift.to_string()));
    }
    if def.opcode_order.contains(&139) || def.boughtlink != 0 {
        p.push((139, "boughtlink".into(), obj_name(registry, def.boughtlink)));
    }
    if def.opcode_order.contains(&140) || def.boughttemplate != 0 {
        p.push((
            140,
            "boughttemplate".into(),
            obj_name(registry, def.boughttemplate),
        ));
    }
    if let Some(params) = &def.params {
        // Decode all param entries from raw opcode data (preserves earlier
        // values for duplicate keys that were overwritten in the HashMap).
        let raw_param_values = decode_raw_params(&def.raw_opcodes);

        if !def.param_order.is_empty() && !raw_param_values.is_empty() {
            for (i, k) in def.param_order.iter().enumerate() {
                let pname = param_name(registry, *k as u32);
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
                let pname = param_name(registry, key as u32);
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
    super::reorder_by_opcode_with_raw_fn(&def.opcode_order, p, &def.raw_opcodes, decode_obj_raw)
}

/// Decode all param entries from the raw op249 data, preserving order
/// and individual values (even for duplicate keys that were overwritten
/// in the HashMap during normal decode).
fn decode_raw_params(raw_opcodes: &[(u8, Vec<u8>)]) -> Vec<crate::cache647::provider::ParamValue> {
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
        // Skip key (3 bytes)
        pos += 3;
        if is_string {
            let start = pos;
            while pos < data.len() && data[pos] != 0 {
                pos += 1;
            }
            let s = String::from_utf8_lossy(&data[start..pos]).to_string();
            pos += 1; // skip NUL
            result.push(crate::cache647::provider::ParamValue::String(
                s.into_boxed_str(),
            ));
        } else {
            if pos + 3 < data.len() {
                let v =
                    i32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                result.push(crate::cache647::provider::ParamValue::Int(v));
            }
            pos += 4;
        }
    }
    result
}

/// Decode raw OBJ opcode bytes back to a (key, value) text pair.
fn decode_obj_raw(opcode: u8, raw: &[u8]) -> Option<(String, String)> {
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
    let r_i32 = |pos: usize| -> i32 {
        if pos + 3 < raw.len() {
            i32::from_be_bytes([raw[pos], raw[pos + 1], raw[pos + 2], raw[pos + 3]])
        } else {
            0
        }
    };
    let r_str = |pos: usize| -> String {
        let mut end = pos;
        while end < raw.len() && raw[end] != 0 {
            end += 1;
        }
        String::from_utf8_lossy(&raw[pos..end]).to_string()
    };

    match opcode {
        1 => Some(("model".into(), r_u16(0).to_string())),
        4 => Some(("2dzoom".into(), r_u16(0).to_string())),
        5 => Some(("2dxan".into(), r_u16(0).to_string())),
        6 => Some(("2dyan".into(), r_u16(0).to_string())),
        7 => Some(("2dxof".into(), r_i16(0).to_string())),
        8 => Some(("2dyof".into(), r_i16(0).to_string())),
        12 => Some(("cost".into(), r_i32(0).to_string())),
        18 => Some(("multistack".into(), r_u16(0).to_string())),
        23 => Some(("manwear".into(), r_u16(0).to_string())),
        24 => Some(("manwear2".into(), r_u16(0).to_string())),
        25 => Some(("womanwear".into(), r_u16(0).to_string())),
        26 => Some(("womanwear2".into(), r_u16(0).to_string())),
        78 => Some(("manwear3".into(), r_u16(0).to_string())),
        79 => Some(("womanwear3".into(), r_u16(0).to_string())),
        90 => Some(("manhead".into(), r_u16(0).to_string())),
        91 => Some(("womanhead".into(), r_u16(0).to_string())),
        92 => Some(("manhead2".into(), r_u16(0).to_string())),
        93 => Some(("womanhead2".into(), r_u16(0).to_string())),
        95 => Some(("2dzan".into(), r_u16(0).to_string())),
        96 => Some(("dummyobj".into(), r_u8(0).to_string())),
        97 => Some(("certlink".into(), r_u16(0).to_string())),
        98 => Some(("certtemplate".into(), r_u16(0).to_string())),
        110 => Some(("resizex".into(), r_u16(0).to_string())),
        111 => Some(("resizey".into(), r_u16(0).to_string())),
        112 => Some(("resizez".into(), r_u16(0).to_string())),
        113 => Some(("ambient".into(), r_i8(0).to_string())),
        114 => Some(("contrast".into(), r_i8(0).to_string())),
        115 => Some(("team".into(), r_u8(0).to_string())),
        121 => Some(("lentlink".into(), r_u16(0).to_string())),
        122 => Some(("lenttemplate".into(), r_u16(0).to_string())),
        134 => Some(("picksizeshift".into(), r_u8(0).to_string())),
        139 => Some(("boughtlink".into(), r_u16(0).to_string())),
        140 => Some(("boughttemplate".into(), r_u16(0).to_string())),
        // String opcodes — needed for duplicate-opcode roundtrip
        30..=34 => Some((format!("op{}", opcode - 29), r_str(0))),
        35..=39 => Some((format!("iop{}", opcode - 34), r_str(0))),
        // Flag opcodes (no payload)
        11 => Some(("stackable".into(), "yes".into())),
        16 => Some(("members".into(), "yes".into())),
        65 => Some(("stockmarket".into(), "yes".into())),
        _ => super::decode_raw_to_text(opcode, raw),
    }
}

/// Grams from a `weight=` value as 289 writes it: a number and a unit, `g`,
/// `kg`, `oz` or `lb` (`500g`, `1.5kg`). The same rule as 289's packer
/// (`pack/config/obj.rs`), including refusing a bare number or a weight past
/// what the opcode holds: content that names no unit is a mistake to fix,
/// not a guess to make.
///
/// # Panics
/// On a value without a known unit, or outside -32768..=32767 grams.
fn parse_weight(value: &str) -> i16 {
    let grams: f64 = if let Some(v) = value.strip_suffix("kg") {
        v.parse::<f64>().unwrap_or(f64::NAN) * 1000.0
    } else if let Some(v) = value.strip_suffix("oz") {
        v.parse::<f64>().unwrap_or(f64::NAN) * 28.3495
    } else if let Some(v) = value.strip_suffix("lb") {
        v.parse::<f64>().unwrap_or(f64::NAN) * 453.592
    } else if let Some(v) = value.strip_suffix('g') {
        v.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        panic!("Invalid weight value: {value}");
    };
    if grams.is_nan() || !(-32768.0..=32767.0).contains(&grams) {
        panic!("Weight out of range: {value}");
    }
    grams as i16
}

#[cfg(test)]
mod weight_tests {
    use super::parse_weight;

    #[test]
    fn weights_parse_as_289_writes_them() {
        assert_eq!(parse_weight("500g"), 500);
        assert_eq!(parse_weight("1.5kg"), 1500);
        assert_eq!(parse_weight("-5g"), -5);
        assert_eq!(parse_weight("1oz"), 28);
        assert_eq!(parse_weight("2lb"), 907);
    }

    #[test]
    #[should_panic(expected = "Invalid weight value")]
    fn a_bare_number_is_refused() {
        parse_weight("500");
    }
}
