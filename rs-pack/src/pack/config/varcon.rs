use super::config_entry;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::PackRegistry;
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_script_var_type;
use crate::types::VarPlayerScope;
use anyhow::Result;
use std::collections::HashMap;

/// The server's varcon table: each controller var's type, scope and name.
/// The cache's own table holds them empty.
pub fn pack_varcons(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedData> {
    let pack = &registry.varcon;
    let configs = parse_config_sections_cached(file_cache, "varcon", constants)?;

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry("varcon", pack, &configs, id) else {
            server.finish_entry();
            continue;
        };

        for (key, value) in props {
            match key.as_str() {
                // 1
                "type" => parse_script_var_type(value, |v| {
                    server.p1(1);
                    server.p1(v as u8);
                }),

                // 2
                "scope" => {
                    let scope = VarPlayerScope::from_config_str(value);
                    if scope != VarPlayerScope::Temp {
                        server.p1(2);
                        server.p1(scope as u8);
                    }
                }

                // not found
                _ => panic!("Unrecognized varcon config key: {key}"),
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
    }

    Ok(server)
}
