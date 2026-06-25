use crate::pack::config::param::parse_params;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

pub fn pack_structs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
) -> Result<PackedFile> {
    let r#struct = &registry.r#struct;

    let files = file_cache.collect("struct");
    debug!("  Found {} .struct files", files.len());

    let configs = parse_config_sections_cached(file_cache, "struct", constants);
    debug!("  Parsed {} struct configs", configs.len());

    let mut server = PackedData::new(r#struct.max);

    for id in 0..r#struct.max {
        server.start_entry();

        let Some(debugname) = r#struct.get_by_id(id) else {
            panic!("Unknown struct id: {}", id);
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown struct config: {}", debugname);
        };

        for (key, _) in props {
            match key.as_str() {
                // 249
                "param" => {} // handled at the end

                // not found
                _ => panic!("Unrecognized struct config key: {key}"),
            }
        }

        // handle 249
        parse_params(registry, param_types, &mut server, props, debugname);

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
