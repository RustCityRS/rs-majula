use super::config_entry;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackFile, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_script_var_type;
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_vars(
    file_cache: &FileCache,
    pack: &PackFile,
    kind: &str,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let configs = parse_config_sections_cached(file_cache, kind, constants)?;

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry(kind, pack, &configs, id) else {
            server.finish_entry();
            continue;
        };

        for (key, value) in props {
            match key.as_str() {
                // 1
                "type" => parse_script_var_type(value, |v| {
                    server.p1(1);
                    server.p1(v as u8);
                }),

                // not found
                _ => panic!("Unrecognized {kind} config key: {key}"),
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
