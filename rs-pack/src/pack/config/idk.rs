use crate::config_crc;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::colour::{RecolType, rgb15_to_hsl16};
use crate::pack::util::{parse_bool, parse_model_kind, parse_recol};
use crate::types::BodyType;
use anyhow::Result;
use rs_io::crc;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_idks(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    verify: bool,
) -> Result<PackedFile> {
    let pack = &registry.idk;

    let files = file_cache.collect("idk");
    debug!("  Found {} .idk files", files.len());

    let configs = parse_config_sections_cached(file_cache, "idk", constants);
    debug!("  Parsed {} idk configs", configs.len());

    let mut server = PackedData::new(pack.max);
    let mut client = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();
        client.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown idk id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown idk config: {debugname}");
        };

        let mut models: Vec<(usize, u16)> = Vec::new();
        let mut head_models: Vec<(usize, u16)> = Vec::new();
        let mut recol_s: Vec<(usize, u16)> = Vec::new();
        let mut recol_d: Vec<(usize, u16)> = Vec::new();

        for (key, value) in props {
            match key.as_str() {
                // 1
                "type" => {
                    let v = BodyType::from_config_str(value);
                    client.p1(1);
                    client.p1(v as u8);
                    server.p1(1);
                    server.p1(v as u8);
                }

                // 2
                _ if key.starts_with("model") => {
                    parse_model_kind(registry, "model", key, value, |v| models.push((v.0, v.1)))
                }

                // 3
                "disable" => parse_bool(value, |v| {
                    if v {
                        client.p1(3);
                        server.p1(3);
                    }
                }),

                // 40-59
                _ if key.starts_with("recol") => parse_recol(key, value, |v| match v {
                    RecolType::S(idx, v) => recol_s.push((idx, v)),
                    RecolType::D(idx, v) => recol_d.push((idx, v)),
                }),

                // 60-69
                _ if key.starts_with("head") => {
                    parse_model_kind(registry, "head", key, value, |v| {
                        head_models.push((v.0, v.1))
                    })
                }

                // not found
                _ => panic!("Unrecognized idk config key: {key}"),
            }
        }

        // handle 40-49
        if !recol_s.is_empty() {
            for (idx, v) in recol_s {
                client.p1((39 + idx) as u8);
                client.p2(rgb15_to_hsl16(v));
                server.p1((39 + idx) as u8);
                server.p2(rgb15_to_hsl16(v));
            }
        }

        // handle 50-59
        if !recol_d.is_empty() {
            for (idx, v) in recol_d {
                client.p1((49 + idx) as u8);
                client.p2(rgb15_to_hsl16(v));
                server.p1((49 + idx) as u8);
                server.p2(rgb15_to_hsl16(v));
            }
        }

        // handle 60-69
        if !head_models.is_empty() {
            for (idx, v) in head_models {
                client.p1((59 + idx) as u8);
                client.p2(v);
                server.p1((59 + idx) as u8);
                server.p2(v);
            }
        }

        // handle 2
        if !models.is_empty() {
            client.p1(2);
            client.p1(models.len() as u8);
            server.p1(2);
            server.p1(models.len() as u8);
            for (_, v) in models {
                client.p2(v);
                server.p2(v);
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
        client.finish_entry();
    }

    if verify {
        let crc = crc::getcrc(&client.dat, 0, client.dat.len());
        let expected = config_crc::IDK;
        if crc != expected {
            panic!("CRC mismatch ['idk']: Got: {crc}, Expected: {expected}");
        }
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
