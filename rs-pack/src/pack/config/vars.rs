use super::config_entry;
#[cfg(since_647)]
use super::{client_table, is_string_var};
#[cfg(since_647)]
use crate::pack::pack::ConfigSections;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackFile, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_script_var_type;
use anyhow::Result;
#[cfg(since_647)]
use anyhow::ensure;
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

    #[cfg(before_647)]
    let client = None;

    #[cfg(since_647)]
    let client = Some(if kind == "vars" {
        client_var_table(&configs, pack, false)?
    } else {
        client_table(file_cache, kind, pack, constants)?
    });

    Ok(PackedFile { server, client })
}

#[cfg(since_647)]
pub fn pack_string_vars(
    file_cache: &FileCache,
    pack: &PackFile,
    kind: &str,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let configs = parse_config_sections_cached(file_cache, kind, constants)?;
    Ok(PackedFile {
        server: PackedData::default(),
        client: Some(client_var_table(&configs, pack, true)?),
    })
}

#[cfg(since_647)]
fn client_var_table(
    configs: &ConfigSections,
    pack: &PackFile,
    strings: bool,
) -> Result<PackedData> {
    if strings {
        for (name, props) in configs {
            if is_string_var(props) {
                ensure!(
                    pack.get_by_debugname(name).is_some(),
                    "No string variable id for [{name}]"
                );
            }
        }
    }
    let count = pack.client_count();
    let mut client = PackedData::new(count);
    for id in 0..count {
        if pack
            .get_by_id(id)
            .and_then(|name| configs.get(name))
            .is_some_and(|props| is_string_var(props) == strings)
        {
            client.start_entry();
            client.finish_entry();
        } else {
            client.skip_entry();
        }
    }
    Ok(client)
}
