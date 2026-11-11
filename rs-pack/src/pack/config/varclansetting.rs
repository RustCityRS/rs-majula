use super::client_table_with;
use crate::pack::pack::FileCache;
use crate::pack::pack_registry::{PackFile, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::{parse_number, parse_ref, parse_script_var_type};
use anyhow::Result;
use std::collections::HashMap;

pub fn pack_varclansettings(
    file_cache: &FileCache,
    pack: &PackFile,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let client = client_table_with(
        file_cache,
        "varclansetting",
        pack,
        constants,
        |client, _, props| {
            for (key, value) in props {
                match key.as_str() {
                    // 1
                    "type" => parse_script_var_type(value, |v| {
                        client.p1(1);
                        client.p1(v as u8);
                    }),

                    // 2
                    "basevar" => parse_ref(pack, "varclansetting", value, |v| {
                        client.p1(2);
                        client.p2(v);
                    }),
                    "startbit" => parse_number(value, |v| client.p1(v)),
                    "endbit" => parse_number(value, |v| client.p1(v)),

                    // not found
                    _ => panic!("Unrecognized varclansetting config key: {key}"),
                }
            }
        },
    )?;
    Ok(PackedFile {
        server: PackedData::default(),
        client: Some(client),
    })
}
