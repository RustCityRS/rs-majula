use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache::param::ParamTypeRaw;
use crate::cache::provider::CacheType;
use crate::pack::pack_registry::PackRegistry;

const ARCHIVE_CONFIG: u8 = 2;
const GROUP_PARAM: u32 = 11;

pub fn unpack_params(cache_dir: &Path, registry: &PackRegistry, output_dir: &Path) -> Result<()> {
    // Write ALL cache entries to all.param — never skip entries that have
    // specific .param files. The param packer merges specific-file
    // server-only fields (type/default) on top of the all.param client
    // entries, so both can coexist without cross-file duplicate panics.
    let mut store = CacheStore::open(cache_dir)?;
    let Some(group) = store.read_group(ARCHIVE_CONFIG, GROUP_PARAM)? else {
        info!("  Archive 2 group {GROUP_PARAM} (param) absent — skipping");
        return Ok(());
    };

    let out_path = output_dir.join("all.param");
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
        let mut def = ParamTypeRaw::new(entry.id as u16);
        def.decode(&mut Packet::from(entry.data.to_vec()));

        let base = registry
            .param
            .get_by_id(entry.id)
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("param_{}", entry.id));
        let section = if seen.contains_key(&base) {
            format!("{base}__{}", entry.id)
        } else {
            base
        };
        seen.insert(section.clone(), entry.id);

        writeln!(file, "[{section}]")?;
        for (k, v) in emit_param_props(&def, registry) {
            writeln!(file, "{k}={v}")?;
        }
        writeln!(file)?;
        written += 1;
    }

    info!("  Wrote {written} param configs to {}", out_path.display());
    Ok(())
}

pub fn emit_param_props(def: &ParamTypeRaw, _registry: &PackRegistry) -> Vec<(String, String)> {
    let mut p: Vec<(u8, String, String)> = Vec::new();
    if def.opcode_order.contains(&1) {
        p.push((1, "type".into(), char_to_type_name(def.var_type as u8)));
    }
    if def.opcode_order.contains(&2) {
        p.push((2, "defaultint".into(), def.default_int.to_string()));
    }
    if !def.autodisable {
        p.push((4, "autodisable".into(), "no".into()));
    }
    if let Some(ds) = &def.default_str {
        p.push((5, "defaultstring".into(), ds.to_string()));
    }
    super::reorder_by_opcode(&def.opcode_order, p)
}

pub(super) fn char_to_type_name(c: u8) -> String {
    let name: Option<&'static str> = match c {
        b'i' => Some("int"),
        b's' => Some("string"),
        b'c' => Some("coord"),
        b'o' => Some("obj"),
        b'O' => Some("namedobj"),
        b'n' => Some("npc"),
        b'l' => Some("loc"),
        b'I' => Some("component"),
        b'1' => Some("boolean"),
        b'g' => Some("enum"),
        b'J' => Some("struct"),
        b'S' => Some("stat"),
        b'A' => Some("seq"),
        b'P' => Some("synth"),
        b'v' => Some("inv"),
        b't' => Some("spotanim"),
        b'V' => Some("varp"),
        b'm' => Some("model"),
        0xb5 => Some("mapelement"),
        b'y' => Some("category"),
        b'K' => Some("idkit"),
        b'p' => Some("player_uid"),
        b'N' => Some("npc_uid"),
        0xd0 => Some("dbrow"),
        _ => None,
    };
    match name {
        Some(n) => n.to_string(),
        None => format!("byte:0x{c:02x}"),
    }
}
