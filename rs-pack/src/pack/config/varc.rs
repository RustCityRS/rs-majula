use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackFile, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_script_var_type;
use crate::types::VarPlayerScope;
use anyhow::{Context, Result};
use std::collections::{BTreeMap, HashMap};

pub fn pack_varcs(
    file_cache: &FileCache,
    pack: &PackFile,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    pack_table(file_cache, pack, constants, false)
}

pub fn pack_varc_strings(
    file_cache: &FileCache,
    pack: &PackFile,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    pack_table(file_cache, pack, constants, true)
}

/// `.varc` holds both of the cache's varc tables: `type=string` is the string
/// table, which implies its type, and anything else the other.
fn pack_table(
    file_cache: &FileCache,
    pack: &PackFile,
    constants: &HashMap<String, String>,
    strings: bool,
) -> Result<PackedFile> {
    let configs = file_cache.client_configs("varc", constants)?;
    let mut entries = BTreeMap::new();
    for (name, props) in &configs {
        if props.iter().any(|(k, v)| k == "type" && v == "string") == strings {
            let id = pack
                .get_by_debugname(name)
                .with_context(|| format!("No varc id for [{name}]"))?;
            entries.insert(id, props);
        }
    }
    let count = entries.last_key_value().map_or(0, |(&id, _)| id + 1);
    let mut client = PackedData::new(count);
    for id in 0..count {
        let Some(props) = entries.get(&id) else {
            client.skip_entry();
            continue;
        };
        client.start_entry();
        for (key, value) in *props {
            match key.as_str() {
                // 1
                "type" if !strings => parse_script_var_type(value, |v| {
                    client.p1(1);
                    client.p1(v as u8);
                }),
                "type" => {}

                // 2
                "scope" => {
                    if VarPlayerScope::from_config_str(value) != VarPlayerScope::Temp {
                        client.p1(2);
                    }
                }

                // not found
                _ => panic!("Unrecognized varc config key: {key}"),
            }
        }
        client.finish_entry();
    }
    Ok(PackedFile {
        server: PackedData::default(),
        client: Some(client),
    })
}
