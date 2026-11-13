use super::config_entry;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
#[cfg(since_647)]
use crate::pack::util::parse_number;
use crate::pack::util::parse_ref;
use crate::pack::util::{parse_bool, parse_hex};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_flos(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.flo;

    let configs = parse_config_sections_cached(file_cache, "flo", constants)?;

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some((debugname, props)) = config_entry("flo", pack, &configs, id) else {
            server.finish_entry();
            client.finish_entry();
            continue;
        };

        for (key, value) in props {
            match key.as_str() {
                // 1
                "colour" => parse_hex(value, |v| {
                    client.p1(1);
                    client.p3(v);
                    server.p1(1);
                    server.p3(v);
                }),

                // 2
                "texture" => parse_ref(&registry.texture, "texture", value, |v| {
                    #[cfg(before_647)]
                    let v = v as u8;
                    #[cfg(since_647)]
                    let v = u8::try_from(v).expect("Floor texture exceeds 255");
                    client.p1(2);
                    client.p1(v);
                    server.p1(2);
                    server.p1(v);
                }),

                // 3
                #[cfg(before_647)]
                "overlay" => parse_bool(value, |v| {
                    if v {
                        client.p1(3);
                        server.p1(3);
                    }
                }),

                #[cfg(since_647)]
                "material" => parse_ref(&registry.texture, "texture", value, |v| {
                    server.p1(3);
                    server.p2(v);
                    client.p1(3);
                    client.p2(v);
                }),

                // 5
                "occlude" => parse_bool(value, |v| {
                    if !v {
                        client.p1(5);
                        server.p1(5);
                    }
                }),

                // 7
                #[cfg(since_647)]
                "mapcolour" => parse_hex(value, |v| {
                    server.p1(7);
                    server.p3(v);
                    client.p1(7);
                    client.p3(v);
                }),

                // 8
                #[cfg(since_647)]
                "unknown8" => {
                    server.p1(8);
                    client.p1(8);
                }

                // 9
                #[cfg(since_647)]
                "texturescale" => parse_number(value, |v: u16| {
                    server.p1(9);
                    server.p2(v);
                    client.p1(9);
                    client.p2(v);
                }),

                // 10
                #[cfg(since_647)]
                "hardshadow" => parse_bool(value, |v| {
                    if !v {
                        server.p1(10);
                        client.p1(10);
                    }
                }),

                // 11
                #[cfg(since_647)]
                "priority" => parse_number(value, |v: u8| {
                    server.p1(11);
                    server.p1(v);
                    client.p1(11);
                    client.p1(v);
                }),

                // 12
                #[cfg(since_647)]
                "smoothedges" => {
                    server.p1(12);
                    client.p1(12);
                }

                // 13
                #[cfg(since_647)]
                "waterfogcolour" => parse_hex(value, |v| {
                    server.p1(13);
                    server.p3(v);
                    client.p1(13);
                    client.p3(v);
                }),

                // 14
                #[cfg(since_647)]
                "waterfogscale" => parse_number(value, |v: u8| {
                    server.p1(14);
                    server.p1(v);
                    client.p1(14);
                    client.p1(v);
                }),

                // 16
                #[cfg(since_647)]
                "waterfogoffset" => parse_number(value, |v: u8| {
                    server.p1(16);
                    server.p1(v);
                    client.p1(16);
                    client.p1(v);
                }),

                // not found
                _ => panic!("Unrecognized flo config key: {key}"),
            }
        }

        // 6
        server.p1(6);
        server.pjstr(debugname);
        #[cfg(before_647)]
        if !debugname.starts_with("flo_") {
            // yes, this was originally transmitted!
            client.p1(6);
            client.pjstr(debugname);
        }

        // done
        server.finish_entry();
        client.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
