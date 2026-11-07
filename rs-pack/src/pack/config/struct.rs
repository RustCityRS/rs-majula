use super::config_entry;
use crate::pack::config::param::parse_params;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_structs(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    param_types: &HashMap<String, String>,
) -> Result<PackedFile> {
    let r#struct = &registry.r#struct;

    let configs = parse_config_sections_cached(file_cache, "struct", constants)?;

    let mut server = PackedData::new(r#struct.max);
    #[cfg(since_647)]
    let client_configs = file_cache.client_configs("struct", constants)?;
    #[cfg(since_647)]
    let client_count = file_cache
        .client_count("struct", r#struct, constants)?
        .unwrap_or(0);
    #[cfg(since_647)]
    let mut client = PackedData::new(client_count);

    for id in 0..r#struct.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry("struct", r#struct, &configs, id) else {
            server.skip_entry();
            #[cfg(since_647)]
            if id < client_count {
                client.skip_entry();
            }
            continue;
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

        #[cfg(since_647)]
        if id < client_count {
            if let Some(props) = client_configs.get(debugname) {
                client.start_entry();
                parse_params(registry, param_types, &mut client, props, debugname);
                client.finish_entry();
            } else {
                client.skip_entry();
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
        #[cfg(before_647)]
        client: None,
        #[cfg(since_647)]
        client: Some(client),
    })
}
