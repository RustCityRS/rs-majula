use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use super::param::char_to_type_name;
use crate::cache::r#enum::EnumTypeRaw;
use crate::cache::provider::CacheType;
use crate::pack::pack_registry::PackRegistry;
use crate::types::{ParamValue, ScriptVarType};

const ARCHIVE_ENUM: u8 = 17;
const STRIDE_ENUM: u32 = 256;

pub fn unpack_enums(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let group_count = store.group_count(ARCHIVE_ENUM).unwrap_or(0);

    let out_path = output_dir.join("all.enum");
    let mut file = std::fs::File::create(&out_path)?;

    let mut written = 0u32;

    for gid in 0..group_count {
        let Some(group) = store.read_group(ARCHIVE_ENUM, gid)? else {
            continue;
        };

        let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
        sorted.sort_by_key(|e| e.id);

        for entry in sorted {
            if entry.data.is_empty() {
                continue;
            }
            let global_id = gid * STRIDE_ENUM + entry.id;
            let mut def = EnumTypeRaw::new(global_id as u16);
            let mut buf = Packet::from(entry.data.to_vec());
            def.decode(&mut buf);

            let fallback = format!("enum_{global_id}");
            let name = registry.r#enum.get_by_id(global_id).unwrap_or(&fallback);

            writeln!(file, "[{name}]")?;
            for (k, v) in emit_enum_props(&def) {
                writeln!(file, "{k}={v}")?;
            }
            writeln!(file)?;
            written += 1;
        }
    }

    info!("  Wrote {written} enum configs to {}", out_path.display());
    Ok(())
}

pub fn emit_enum_props(def: &EnumTypeRaw) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();

    // Emit opcodes in the original decode order. This is critical because
    // the Jagex cache sometimes has entries (op 5/6) before defaults (op 3/4),
    // and the packer must reproduce the exact same byte order.
    let mut entries_emitted = false;
    for &op in &def.opcode_order {
        match op {
            1 => {
                props.push(("inputtype".into(), char_to_type_name(def.inputtype as u8)));
            }
            2 => {
                props.push(("outputtype".into(), char_to_type_name(def.outputtype as u8)));
            }
            3 => {
                // Client defaults to "null", so any op 3 present in the binary
                // is meaningful — even if the string is empty.
                props.push((
                    "defaultstring".into(),
                    def.default_str.as_deref().unwrap_or("").to_string(),
                ));
            }
            4 => {
                // Op 4 was explicitly present — always emit.
                props.push(("defaultint".into(), def.default_int.to_string()));
            }
            5 | 6 if !entries_emitted => {
                entries_emitted = true;
                // Entries — emit in decode order, using ordered_entries
                // which preserves duplicates with their original values.
                for (key, value) in &def.ordered_entries {
                    match value {
                        ParamValue::String(s) => {
                            props.push(("val".into(), format!("{key},{s}")));
                        }
                        ParamValue::Int(v) => {
                            props.push(("val".into(), format!("{key},{v}")));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    // Fallback: if opcode_order is empty (e.g. hand-written configs), use
    // the legacy fixed order.
    if def.opcode_order.is_empty() {
        if def.inputtype != ScriptVarType::Int {
            props.push(("inputtype".into(), char_to_type_name(def.inputtype as u8)));
        }
        if def.outputtype != ScriptVarType::Int {
            props.push(("outputtype".into(), char_to_type_name(def.outputtype as u8)));
        }
        if def.default_str.as_deref().is_some_and(|s| !s.is_empty()) {
            props.push((
                "defaultstring".into(),
                def.default_str.as_deref().unwrap_or("").to_string(),
            ));
        }
        if def.default_int != 0 {
            props.push(("defaultint".into(), def.default_int.to_string()));
        }
        for (key, value) in &def.ordered_entries {
            match value {
                ParamValue::String(s) => {
                    props.push(("val".into(), format!("{key},{s}")));
                }
                ParamValue::Int(v) => {
                    props.push(("val".into(), format!("{key},{v}")));
                }
            }
        }
    }
    props
}
