use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::provider::CacheType;
use crate::cache::r#struct::StructTypeRaw;
use crate::pack::pack_registry::PackRegistry;
use crate::types::ParamValue;

const ARCHIVE_CONFIG: u8 = 2;
const GROUP_STRUCT: u32 = 26;

pub fn unpack_structs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let Some(group) = store.read_group(ARCHIVE_CONFIG, GROUP_STRUCT)? else {
        info!("  Archive 2 group {GROUP_STRUCT} (struct) absent — skipping");
        return Ok(());
    };

    let out_path = output_dir.join("all.struct");
    let mut file = std::fs::File::create(&out_path)?;

    // Sort entries by id for deterministic output.
    let mut sorted: Vec<&rs_dat2::Entry> = group.entries.iter().collect();
    sorted.sort_by_key(|e| e.id);

    let mut seen: HashMap<String, u32> = HashMap::new();
    let mut written = 0u32;

    for entry in sorted {
        if entry.data.is_empty() {
            continue;
        }
        let mut def = StructTypeRaw::new(entry.id as u16);
        def.decode(&mut Packet::from(entry.data.to_vec()));

        let base = registry
            .r#struct
            .get_by_id(entry.id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("struct_{}", entry.id));
        let section = if seen.contains_key(&base) {
            format!("{base}__{}", entry.id)
        } else {
            base
        };
        seen.insert(section.clone(), entry.id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_struct_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} struct configs to {}", out_path.display());
    Ok(())
}

pub fn emit_struct_props(def: &StructTypeRaw, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    for (key, val) in def.params.as_deref().unwrap_or_default() {
        let pname = registry
            .param
            .get_by_id(*key as u32)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("param_{key}"));
        match val {
            ParamValue::Int(i) => {
                props.push(("param".into(), format!("{pname},{i}")));
            }
            ParamValue::String(s) => {
                props.push(("param".into(), format!("{pname},{s}")));
            }
        }
    }
    props
}
