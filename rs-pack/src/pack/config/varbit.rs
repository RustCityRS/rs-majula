#[cfg(since_254)]
use std::collections::HashMap;

#[cfg(since_254)]
use crate::config_crc;
#[cfg(since_254)]
use crate::pack::pack::{FileCache, parse_config_sections_cached};
#[cfg(since_254)]
use crate::pack::pack_registry::{PackRegistry, PackedFile};
#[cfg(since_254)]
use crate::pack::packed_data::PackedData;
#[cfg(since_254)]
use crate::pack::util::parse_number;
#[cfg(since_254)]
use crate::pack::util::parse_varp;
#[cfg(since_254)]
use anyhow::Result;
#[cfg(since_254)]
use rs_io::crc;
#[cfg(since_254)]
use tracing::debug;

#[cfg(since_254)]
pub fn pack_varbits(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.varbit;

    let files = file_cache.collect("varbit");
    debug!("  Found {} .varbit files", files.len());

    let configs = parse_config_sections_cached(file_cache, "varbit", constants);
    debug!("  Parsed {} varbit configs", configs.len());

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown varbit id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown varbit config: {debugname}");
        };

        let mut basevar = None;
        let mut startbit = None;
        let mut endbit = None;

        for (key, value) in props {
            match key.as_str() {
                // 1
                "basevar" => parse_varp(registry, value, |v| {
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
            client.p1(1);
            client.p2(basevar);
            client.p1(startbit);
            client.p1(endbit);
            server.p1(1);
            server.p2(basevar);
            server.p1(startbit);
            server.p1(endbit);
        }

        /*if !debugname.starts_with("varbit_") {
            // yes, this was originally transmitted!
            client.p1(10);
            client.pjstr(debugname);
            server.p1(10);
            server.pjstr(debugname);
        }*/

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        client.finish_entry();
    }

    if verify {
        let crc = crc::getcrc(&client.dat, 0, client.dat.len());
        let expected = config_crc::VARBIT;
        if crc != expected {
            panic!("CRC mismatch ['varbit']: Got: {crc}, Expected: {expected}");
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
