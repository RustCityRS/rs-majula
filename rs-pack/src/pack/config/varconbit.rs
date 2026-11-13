use super::varbit::bit_range;
use super::{config_entry, is_string_var};
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackId, PackRegistry};
use crate::pack::packed_data::PackedData;
use anyhow::{Context, Result, ensure};
use std::collections::HashMap;

pub fn pack_varconbits(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedData> {
    let pack = &registry.varconbit;
    let configs = parse_config_sections_cached(file_cache, "varconbit", constants)?;
    let varcons = parse_config_sections_cached(file_cache, "varcon", constants)?;
    let mut server = PackedData::new(pack.max);
    for id in 0..pack.max {
        server.start_entry();
        if let Some((name, props)) = config_entry("varconbit", pack, &configs, id) {
            if let Some((basevar, startbit, endbit)) = bit_range(props, &registry.varcon, "varcon")?
            {
                let base = registry
                    .varcon
                    .get_by_id(basevar as PackId)
                    .and_then(|name| varcons.get(name))
                    .with_context(|| format!("[{name}]: no base varcon {basevar}"))?;
                ensure!(
                    !is_string_var(base),
                    "[{name}]: base varcon must not be a string"
                );
                server.p1(1);
                server.p2(basevar);
                server.p1(startbit);
                server.p1(endbit);
            }
            server.p1(250);
            server.pjstr(name);
        }
        server.finish_entry();
    }
    Ok(server)
}
