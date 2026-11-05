use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::inv::InvTypeRaw;
use crate::cache::provider::CacheType;
use crate::pack::pack_registry::PackRegistry;
use crate::types::InvScope;

const ARCHIVE_CONFIG: u8 = 2;
const GROUP_INV: u32 = 5;

pub fn unpack_invs(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    let mut store = CacheStore::open(cache_dir)?;
    let Some(group) = store.read_group(ARCHIVE_CONFIG, GROUP_INV)? else {
        info!("  Archive 2 group {GROUP_INV} (inv) absent — skipping");
        return Ok(());
    };

    let out_path = output_dir.join("all.inv");
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
        let mut def = InvTypeRaw::new(entry.id as u16);
        let mut buf = Packet::from(entry.data.to_vec());
        def.decode(&mut buf);

        let base = registry
            .inv
            .get_by_id(entry.id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("inv_{}", entry.id));
        let section = if seen.contains_key(&base) {
            format!("{base}__{}", entry.id)
        } else {
            base
        };
        seen.insert(section.clone(), entry.id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_inv_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} inv configs to {}", out_path.display());
    Ok(())
}

pub fn emit_inv_props(def: &InvTypeRaw, registry: &PackRegistry) -> Vec<(String, String)> {
    let mut props: Vec<(String, String)> = Vec::new();
    if def.has_scope {
        let s = match def.scope {
            InvScope::Temp => "temp".to_string(),
            InvScope::Perm => "perm".to_string(),
            InvScope::Shared => "shared".to_string(),
        };
        props.push(("scope".into(), s));
    }
    if def.size != 0 {
        props.push(("size".into(), def.size.to_string()));
    }
    if def.stackall {
        props.push(("stackall".into(), "yes".into()));
    }
    let stockobj = def.stockobj.as_deref().unwrap_or_default();
    let stockcount = def.stockcount.as_deref().unwrap_or_default();
    let stockrate = def.stockrate.as_deref().unwrap_or_default();
    if !stockobj.is_empty() {
        for i in 0..stockobj.len() {
            let obj_name = if stockobj[i] == u16::MAX {
                "null".to_string()
            } else {
                registry
                    .obj
                    .get_by_id(stockobj[i] as u32)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("obj_{}", stockobj[i]))
            };
            let value = if stockrate[i] != 0 {
                format!("{},{},{}", obj_name, stockcount[i], stockrate[i])
            } else {
                format!("{},{}", obj_name, stockcount[i])
            };
            props.push((format!("stock{}", i + 1), value));
        }
    }
    if def.restock {
        props.push(("restock".into(), "yes".into()));
    }
    if def.allstock {
        props.push(("allstock".into(), "yes".into()));
    }
    if !def.protect {
        props.push(("protect".into(), "no".into()));
    }
    if def.runweight {
        props.push(("runweight".into(), "yes".into()));
    }
    if def.dummyinv {
        props.push(("dummyinv".into(), "yes".into()));
    }
    props
}
