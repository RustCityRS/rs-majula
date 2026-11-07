use std::collections::HashMap;

use anyhow::Result;

use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;

pub fn pack_flus(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.flu;
    let configs = parse_config_sections_cached(file_cache, "flu", constants)?;
    let max_id = pack.max;

    let mut server = PackedData::new(max_id);
    let mut client = PackedData::new(max_id);

    for id in 0..max_id {
        server.start_entry();
        client.start_entry();

        let section_name = pack.get_by_id(id).unwrap_or("");
        let Some(props) = configs.get(section_name) else {
            server.finish_entry();
            client.finish_entry();
            continue;
        };

        for (key, value) in props {
            match key.as_str() {
                "color" => {
                    let v: u32 = value.parse().expect("Invalid flu value");
                    client.p1(1);
                    client.p3(v as i32);
                }
                "texture" => {
                    let v: u16 = value.parse().expect("Invalid flu value");
                    client.p1(2);
                    client.p2(v);
                }
                "size" => {
                    let v: u16 = value.parse().expect("Invalid flu value");
                    client.p1(3);
                    client.p2(v);
                }
                "op4_clearbool201" => {
                    client.p1(4);
                }
                "op5_clearbool202" => {
                    client.p1(5);
                }
                _ => panic!("Unrecognized flu config key: {key}"),
            }
        }

        server.dat.extend_from_slice(client.entry());
        server.finish_entry();
        client.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
