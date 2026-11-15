use super::client_table_with;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackFile, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_number, parse_ref, parse_script_var_type};
use anyhow::Result;
use std::collections::HashMap;

/// Packs clan settings, or from 742 clan vars, whose bit range goes on `bitcode`.
pub fn pack_clan_vars(
    file_cache: &FileCache,
    kind: &str,
    pack: &PackFile,
    constants: &HashMap<String, String>,
    bitcode: u8,
) -> Result<PackedFile> {
    let client = client_table_with(file_cache, kind, pack, constants, |client, _, props| {
        for (key, value) in props {
            match key.as_str() {
                // 1
                "type" => parse_script_var_type(value, |v| {
                    client.p1(1);
                    client.p1(v as u8);
                }),

                // 2 or 3
                "basevar" => parse_ref(pack, kind, value, |v| {
                    client.p1(bitcode);
                    client.p2(v);
                }),
                "startbit" => parse_number(value, |v| client.p1(v)),
                "endbit" => parse_number(value, |v| client.p1(v)),

                // not found
                _ => panic!("Unrecognized {kind} config key: {key}"),
            }
        }
    })?;
    Ok(PackedFile {
        server: PackedData::default(),
        client: Some(client),
    })
}
