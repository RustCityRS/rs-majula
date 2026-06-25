use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_script_var_type;
use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_varns(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = &registry.varn;

    let files = file_cache.collect("varn");
    debug!("  Found {} .varn files", files.len());

    let configs = parse_config_sections_cached(file_cache, "varn", constants);
    debug!("  Parsed {} varn configs", configs.len());

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some(debugname) = pack.get_by_id(id) else {
            panic!("Unknown varn id: {id}");
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown varn config: {debugname}");
        };

        for (key, value) in props {
            match key.as_str() {
                // 1
                "type" => parse_script_var_type(value, |v| {
                    server.p1(1);
                    server.p1(v as u8);
                }),

                // not found
                _ => panic!("Unrecognized varn config key: {key}"),
            }
        }

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: None,
    })
}
