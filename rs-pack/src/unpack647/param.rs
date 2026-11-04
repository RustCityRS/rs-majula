//! Unpack Param configs from JS5 cache to text config format.
//!
//! Produces `_unpack/all.param` such that `pack::param::pack_param_def`
//! reading the file produces byte-identical cache bytes for every param.
//! Mirrors 2004scape's `tools/pack/config/ParamConfig.ts` text shape, but
//! with distinct keys for the overloaded `default=` (see `pack/param.rs`):
//! `type=`, `defaultint=`, `defaultstring=`, `autodisable=no`.
//!
//! Section header is the registry-mapped name (`registry.param.get_by_id`),
//! falling back to `param_<id>`. Param's cache shape has no DebugName op, so
//! the section is purely a content identifier — bytes don't depend on it.
//!
//! Unlike other config types, param never skips entries that have
//! specific .param files — the packer merges specific-file server-only
//! fields on top of the all.param client entries.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::Result;
use rs_dat2::CacheStore;
use rs_io::Packet;
use tracing::info;

use crate::cache647::{CacheType, ParamType};
use crate::pack::param::emit_param_props;
use crate::pack_registry::PackRegistry;

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
        let mut def = ParamType::new(entry.id as u16);
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
