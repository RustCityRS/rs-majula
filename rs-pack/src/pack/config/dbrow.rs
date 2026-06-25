use std::collections::HashMap;

use crate::ParamValue;
use crate::pack::config::param::*;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackFile, PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_csv_values;
use crate::types::ScriptVarType;
use anyhow::Result;
use tracing::debug;

/// Loaded table schema for resolving column names and types.
struct TableSchema {
    columns: Vec<(String, Vec<u8>, u8)>, // (name, type_chars, props)
}

pub fn pack_dbrows(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let row_pack = PackFile::load(&registry.pack_dir().join("dbrow.pack"))?;
    let table_pack = PackFile::load(&registry.pack_dir().join("dbtable.pack"))?;

    let table_configs = parse_config_sections_cached(file_cache, "dbtable", constants);
    let mut table_schemas: HashMap<String, TableSchema> = HashMap::new();
    for (tname, props) in &table_configs {
        let mut columns: Vec<(String, Vec<u8>, u8)> = Vec::new(); // (name, types, props)
        for (key, value) in props {
            if key == "column" {
                let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
                if !parts.is_empty() {
                    let col_name = parts[0].to_string();
                    let mut col_types = Vec::new();
                    let mut col_props = 0u8;
                    for p in &parts[1..] {
                        if p.chars().all(|c| c.is_uppercase() || c == '_') {
                            if *p == "REQUIRED" {
                                col_props |= 0x1;
                            }
                            if *p == "LIST" {
                                col_props |= 0x2;
                            }
                        } else {
                            col_types.push(ScriptVarType::from_type_name(p) as u8);
                        }
                    }
                    columns.push((col_name, col_types, col_props));
                }
            }
        }
        table_schemas.insert(tname.to_string(), TableSchema { columns });
    }

    let files = file_cache.collect("dbrow");
    debug!("  Found {} .dbrow files", files.len());
    let configs = parse_config_sections_cached(file_cache, "dbrow", constants);
    debug!("  Parsed {} dbrow configs", configs.len());

    let mut server = PackedData::new(row_pack.max);

    for id in 0..row_pack.max {
        server.start_entry();

        let Some(debugname) = row_pack.get_by_id(id) else {
            panic!("Unknown dbrow id: {}", id);
        };

        let Some(props) = configs.get(debugname) else {
            panic!("Unknown dbrow config: {}", debugname);
        };

        let table_name = props
            .iter()
            .find(|(k, _)| k == "table")
            .map(|(_, v)| v.as_str());

        let schema = table_name.and_then(|tn| table_schemas.get(tn));

        // Parse all data lines grouped by column name
        let data_lines: Vec<(&str, Vec<String>)> = props
            .iter()
            .filter(|(k, _)| k == "data")
            .filter_map(|(_, v)| {
                let (col_name, rest) = v.split_once(',')?;
                let values = parse_csv_values(rest);
                Some((col_name.trim(), values))
            })
            .collect();

        if !data_lines.is_empty() {
            let schema =
                schema.ok_or_else(|| anyhow::anyhow!("No table defined for dbrow: {debugname}"))?;

            // Opcode 3: column data - iterate in schema order like TypeScript
            server.p1(3);
            server.p1(schema.columns.len() as u8);

            for (col_idx, (col_name, col_types, col_props)) in schema.columns.iter().enumerate() {
                server.p1(col_idx as u8);
                server.p1(col_types.len() as u8);
                for &t in col_types {
                    server.p1(t);
                }

                // Find all data entries for this column (matching TypeScript's fields filter)
                let fields: Vec<&Vec<String>> = data_lines
                    .iter()
                    .filter(|(n, _)| *n == col_name.as_str())
                    .map(|(_, v)| v)
                    .collect();

                if (col_props & 0x1) != 0 && fields.is_empty() {
                    return Err(anyhow::anyhow!(
                        "{debugname}: {col_name} column is marked REQUIRED, please add data for it"
                    ));
                }

                if (col_props & 0x2) == 0 && fields.len() > 1 {
                    return Err(anyhow::anyhow!(
                        "{debugname}: {col_name} column has multiple data values but is not marked as LIST"
                    ));
                }

                server.p1(fields.len() as u8);
                for values in fields {
                    for (k, val) in values.iter().enumerate() {
                        let type_char = col_types
                            .get(k % col_types.len().max(1))
                            .copied()
                            .unwrap_or(b'i');
                        let svt = ScriptVarType::try_from(type_char)?;
                        let Some(value) = get_param_value_for_type(svt.type_name(), registry, val)
                        else {
                            panic!(
                                "Unknown param type {} for [{debugname}] for value [{val}]",
                                char::from(type_char)
                            );
                        };
                        match value {
                            ParamValue::Int(value) => {
                                server.p4(value);
                            }
                            ParamValue::String(value) => {
                                server.pjstr(&value);
                            }
                        }
                    }
                }
            }

            server.p1(255); // end columns
        }

        // Opcode 4: table reference - after opcode 3, matching TypeScript order
        if let Some(tn) = table_name {
            if let Some(tid) = table_pack.get_by_debugname(tn) {
                server.p1(4);
                server.p2(tid);
            }
        }

        if !debugname.is_empty() {
            server.p1(250);
            server.pjstr(debugname);
        }

        server.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: None,
    })
}
