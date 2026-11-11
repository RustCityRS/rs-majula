#[cfg(since_647)]
use super::client_table_with;
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

    for id in 0..r#struct.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry("struct", r#struct, &configs, id) else {
            server.skip_entry();
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

        // 250
        server.p1(250);
        server.pjstr(debugname);

        // done
        server.finish_entry();
    }

    #[cfg(before_647)]
    let client = None;
    #[cfg(since_647)]
    let client = Some(client_table_with(
        file_cache,
        "struct",
        r#struct,
        constants,
        |client, name, props| parse_params(registry, param_types, client, props, name),
    )?);

    Ok(PackedFile { server, client })
}
