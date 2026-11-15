use super::is_string_var;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
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
    let configs = parse_config_sections_cached(file_cache, "varc", constants)?;
    let mut entries = BTreeMap::new();
    for (name, props) in &configs {
        if is_string_var(props) == strings {
            let id = pack
                .get_by_debugname(name)
                .with_context(|| format!("No varc id for [{name}]"))?;
            entries.insert(id, props);
        }
    }
    let count = entries.last_key_value().map_or(0, |(&id, _)| id + 1);
    let mut packed = PackedData::new(count);
    #[cfg(all(since_530, before_647))]
    let mut empty = PackedData::new(count);
    for id in 0..count {
        let Some(props) = entries.get(&id) else {
            packed.skip_entry();
            #[cfg(all(since_530, before_647))]
            empty.skip_entry();
            continue;
        };
        packed.start_entry();
        for (key, value) in *props {
            match key.as_str() {
                // 1
                "type" if !strings => parse_script_var_type(value, |v| {
                    packed.p1(1);
                    packed.p1(v as u8);
                }),
                "type" => {}

                // 2
                "scope" => {
                    if VarPlayerScope::from_config_str(value) != VarPlayerScope::Temp {
                        packed.p1(2);
                    }
                }

                // not found
                _ => panic!("Unrecognized varc config key: {key}"),
            }
        }
        packed.finish_entry();
        #[cfg(all(since_530, before_647))]
        {
            empty.start_entry();
            empty.finish_entry();
        }
    }
    // The 530 cache's varc tables are empty records: the types are the server's.
    #[cfg(all(since_530, before_647))]
    let (server, client) = (packed, empty);
    #[cfg(since_647)]
    let (server, client) = (PackedData::default(), packed);
    Ok(PackedFile {
        server,
        client: Some(client),
    })
}
