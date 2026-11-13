use super::{client_table, config_entry};
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_number;
use anyhow::{Result, bail, ensure};
use std::collections::HashMap;

pub fn pack_varobjs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.varobj;

    let configs = parse_config_sections_cached(file_cache, "varobj", constants)?;

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry("varobj", pack, &configs, id) else {
            server.finish_entry();
            continue;
        };

        let mut startbit = None;
        let mut endbit = None;

        for (key, value) in props {
            match key.as_str() {
                "startbit" => parse_number(value, |v: u8| startbit = Some(v)),
                "endbit" => parse_number(value, |v: u8| endbit = Some(v)),

                // not found
                _ => panic!("Unrecognized varobj config key: {key}"),
            }
        }

        // 1
        match (startbit, endbit) {
            (Some(startbit), Some(endbit)) => {
                ensure!(
                    startbit <= endbit && endbit < 32,
                    "[{debugname}]: bits {startbit}..={endbit} are outside an obj's 32"
                );
                server.p1(1);
                server.p1(startbit);
                server.p1(endbit);
            }
            (None, None) => {}
            _ => bail!("[{debugname}]: a varobj needs both startbit and endbit"),
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: Some(client_table(file_cache, "varobj", pack, constants)?),
    })
}
