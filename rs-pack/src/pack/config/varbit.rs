use super::config_entry;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_number, parse_ref};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_varbits(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.varbit;

    let configs = parse_config_sections_cached(file_cache, "varbit", constants)?;

    let mut server = PackedData::new(pack.max);
    #[cfg(before_647)]
    let client_count = pack.max;
    #[cfg(since_647)]
    let client_count = pack.client_count();
    let mut client = PackedData::new(client_count);

    for id in 0..pack.max {
        server.start_entry();
        if id < client_count {
            client.start_entry();
        }

        let Some((debugname, props)) = config_entry("varbit", pack, &configs, id) else {
            server.skip_entry();
            if id < client_count {
                client.skip_entry();
            }
            continue;
        };

        let mut basevar = None;
        let mut startbit = None;
        let mut endbit = None;

        for (key, value) in props {
            match key.as_str() {
                // 1
                "basevar" => parse_ref(&registry.varp, "varp", value, |v| {
                    basevar = Some(v);
                }),
                "startbit" => parse_number(value, |v| startbit = Some(v)),
                "endbit" => parse_number(value, |v| endbit = Some(v)),

                // not found
                _ => panic!("Unrecognized varbit config key: {key}"),
            }
        }

        if let Some(basevar) = basevar
            && let Some(startbit) = startbit
            && let Some(endbit) = endbit
        {
            if id < client_count {
                client.p1(1);
                client.p2(basevar);
                client.p1(startbit);
                client.p1(endbit);
            }
            server.p1(1);
            server.p2(basevar);
            server.p1(startbit);
            server.p1(endbit);
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        if id < client_count {
            client.finish_entry();
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
