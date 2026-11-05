use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::obj::ObjTypeRaw;
use crate::cache::provider::CacheType;
use crate::pack::pack_registry::PackRegistry;
use crate::types::ParamValue;

const ARCHIVE_ITEMS: u8 = 19;
const STRIDE: u32 = 256;

pub fn unpack_objs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;

    let out_path = output_dir.join("all.obj");
    let mut file = std::fs::File::create(&out_path)?;

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    let group_count = store.group_count(ARCHIVE_ITEMS).unwrap_or(0);
    for group_id in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_ITEMS, group_id)? else {
            continue;
        };

        // Sort entries by id within the group for deterministic output.
        let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
        sorted.sort_by_key(|e| e.id);

        for entry in sorted {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = group_id * STRIDE + entry.id;

            let mut def = ObjTypeRaw::new(global_id as u16);
            def.decode(&mut Packet::from(entry.data.to_vec()));

            let base = registry
                .obj
                .get_by_id(global_id)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("obj_{global_id}"));
            let section = if seen.contains_key(&base) {
                format!("{base}__{global_id}")
            } else {
                base
            };
            seen.insert(section.clone(), global_id);

            super::common::write_config_section(
                &mut file,
                &section,
                emit_obj_props(&def, registry),
            )?;
            written += 1;
        }
    }

    info!("  Wrote {written} obj configs to {}", out_path.display());
    Ok(())
}

/// Server-only keys for OBJ configs: fields that exist only in the server
/// stream and have no opcode in the Jagex cache, so a decode of the cache
/// cannot reproduce them. [`update_specific_obj_file`] keeps them verbatim
/// when it refreshes an authored file's cache fields.
///
/// The set is taken from the server-stream branch of
/// [`crate::pack::config::obj::pack_objs`] -- every key it turns into a server opcode
/// and that [`emit_obj_props`] does not emit back.
const OBJ_SERVER_ONLY_KEYS: &[&str] = &[
    "param",
    "paramint",
    "paramstr",
    "desc",
    "wearpos",
    "wearpos2",
    "wearpos3",
    "weight",
    "category",
    "respawnrate",
    "dummyitem",
    "tradeable",
];

/// Update a specific `.obj` file in-place: regenerate client fields from the
/// original cache while preserving server-only additions (`param=...`).
pub fn update_specific_obj_file(
    cache_dir: &Path,
    registry: &PackRegistry,
    file_path: &Path,
) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    super::common::update_specific_file::<ObjTypeRaw, _, _>(
        &mut store,
        file_path,
        ARCHIVE_ITEMS,
        STRIDE,
        OBJ_SERVER_ONLY_KEYS,
        |name| registry.obj.get_by_debugname(name),
        |def| emit_obj_props(def, registry),
    )
}

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
        if registry.obj.get_by_debugname(name) == Some(id as u32) {
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
pub fn emit_obj_props(def: &ObjTypeRaw, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) || def.model != 0 {
        p.push((1, "model".into(), model_name(registry, def.model)));
    }
    if let Some(name) = &def.name {
        p.push((2, "name".into(), name.to_string()));
    }
    if def.opcode_order.contains(&4) || def.zoom2d != 2000 {
        p.push((4, "2dzoom".into(), def.zoom2d.to_string()));
    }
    if def.opcode_order.contains(&5) || def.xan2d != 0 {
        p.push((5, "2dxan".into(), def.xan2d.to_string()));
    }
    if def.opcode_order.contains(&6) || def.yan2d != 0 {
        p.push((6, "2dyan".into(), def.yan2d.to_string()));
    }
    if def.opcode_order.contains(&7) || def.xof2d != 0 {
        p.push((7, "2dxof".into(), def.xof2d.to_string()));
    }
    if def.opcode_order.contains(&8) || def.yof2d != 0 {
        p.push((8, "2dyof".into(), def.yof2d.to_string()));
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
    if def.opcode_order.contains(&23) || def.manwear.unwrap_or(0) != 0 {
        p.push((
            23,
            "manwear".into(),
            model_name(registry, def.manwear.unwrap_or(0)),
        ));
    }
    if def.opcode_order.contains(&24) || def.manwear2.unwrap_or(0) != 0 {
        p.push((
            24,
            "manwear2".into(),
            model_name(registry, def.manwear2.unwrap_or(0)),
        ));
    }
    if def.opcode_order.contains(&25) || def.womanwear.unwrap_or(0) != 0 {
        p.push((
            25,
            "womanwear".into(),
            model_name(registry, def.womanwear.unwrap_or(0)),
        ));
    }
    if def.opcode_order.contains(&26) || def.womanwear2.unwrap_or(0) != 0 {
        p.push((
            26,
            "womanwear2".into(),
            model_name(registry, def.womanwear2.unwrap_or(0)),
        ));
    }
    for (slot, opt) in def.op.as_deref().unwrap_or_default().iter().enumerate() {
        if let Some(text) = opt
            && def.opcode_order.contains(&(30 + slot as u8))
        {
            p.push((30 + slot as u8, format!("op{}", slot + 1), text.to_string()));
        }
    }
    for (slot, opt) in def.iop.as_deref().unwrap_or_default().iter().enumerate() {
        if let Some(text) = opt
            && def.opcode_order.contains(&(35 + slot as u8))
        {
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
    if def.opcode_order.contains(&78) || def.manwear3.unwrap_or(0) != 0 {
        p.push((
            78,
            "manwear3".into(),
            model_name(registry, def.manwear3.unwrap_or(0)),
        ));
    }
    if def.opcode_order.contains(&79) || def.womanwear3.unwrap_or(0) != 0 {
        p.push((
            79,
            "womanwear3".into(),
            model_name(registry, def.womanwear3.unwrap_or(0)),
        ));
    }
    if def.opcode_order.contains(&90) || def.manhead.unwrap_or(0) != 0 {
        p.push((
            90,
            "manhead".into(),
            model_name(registry, def.manhead.unwrap_or(0)),
        ));
    }
    if def.opcode_order.contains(&91) || def.womanhead.unwrap_or(0) != 0 {
        p.push((
            91,
            "womanhead".into(),
            model_name(registry, def.womanhead.unwrap_or(0)),
        ));
    }
    if def.opcode_order.contains(&92) || def.manhead2.unwrap_or(0) != 0 {
        p.push((
            92,
            "manhead2".into(),
            model_name(registry, def.manhead2.unwrap_or(0)),
        ));
    }
    if def.opcode_order.contains(&93) || def.womanhead2.unwrap_or(0) != 0 {
        p.push((
            93,
            "womanhead2".into(),
            model_name(registry, def.womanhead2.unwrap_or(0)),
        ));
    }
    if def.opcode_order.contains(&95) || def.zan2d != 0 {
        p.push((95, "2dzan".into(), def.zan2d.to_string()));
    }
    if def.opcode_order.contains(&96) || def.dummyitem as u8 != 0 {
        p.push((96, "dummyobj".into(), (def.dummyitem as u8).to_string()));
    }
    if def.opcode_order.contains(&97) || def.certlink.unwrap_or(0) != 0 {
        p.push((
            97,
            "certlink".into(),
            obj_name(registry, def.certlink.unwrap_or(0)),
        ));
    }
    if def.opcode_order.contains(&98) || def.certtemplate.unwrap_or(0) != 0 {
        p.push((
            98,
            "certtemplate".into(),
            obj_name(registry, def.certtemplate.unwrap_or(0)),
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
    if def.opcode_order.contains(&110) || def.resizex.unwrap_or(128) != 128 {
        p.push((
            110,
            "resizex".into(),
            def.resizex.unwrap_or(128).to_string(),
        ));
    }
    if def.opcode_order.contains(&111) || def.resizey.unwrap_or(128) != 128 {
        p.push((
            111,
            "resizey".into(),
            def.resizey.unwrap_or(128).to_string(),
        ));
    }
    if def.opcode_order.contains(&112) || def.resizez.unwrap_or(128) != 128 {
        p.push((
            112,
            "resizez".into(),
            def.resizez.unwrap_or(128).to_string(),
        ));
    }
    if def.opcode_order.contains(&113) || def.ambient != 0 {
        p.push((113, "ambient".into(), def.ambient.to_string()));
    }
    if def.opcode_order.contains(&114) || def.contrast != 0 {
        p.push((114, "contrast".into(), def.contrast.to_string()));
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
        || def.manweary != 0
        || def.manwear_zoff != 0
    {
        p.push((
            125,
            "manwearoff".into(),
            format!("{},{},{}", def.manwear_xoff, def.manweary, def.manwear_zoff),
        ));
    }
    if def.opcode_order.contains(&126)
        || def.womanwear_xoff != 0
        || def.womanweary != 0
        || def.womanwear_zoff != 0
    {
        p.push((
            126,
            "womanwearoff".into(),
            format!(
                "{},{},{}",
                def.womanwear_xoff, def.womanweary, def.womanwear_zoff
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
        // Skip key (3 bytes)
        pos += 3;
        if is_string {
            let start = pos;
            while pos < data.len() && data[pos] != 0 {
                pos += 1;
            }
            let s = String::from_utf8_lossy(&data[start..pos]).to_string();
            pos += 1; // skip NUL
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
