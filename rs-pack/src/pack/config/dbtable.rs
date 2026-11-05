use super::config_entry;
use super::param::resolve_typed_value;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackFile, PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_csv_values;
use crate::types::ScriptVarType;
use anyhow::Result;
use std::collections::HashMap;

const PROP_INDEXED: u8 = 0x1;
pub(super) const PROP_REQUIRED: u8 = 0x2;
pub(super) const PROP_LIST: u8 = 0x4;
const PROP_CLIENTSIDE: u8 = 0x8;

pub(super) struct ColumnDef {
    pub name: String,
    pub types: Vec<u8>,
    pub props: u8,
    defaults: Vec<String>,
}

pub fn pack_dbtables(
    file_cache: &FileCache,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
) -> Result<PackedFile> {
    let pack = PackFile::load(&registry.pack_dir().join("dbtable.pack"))?;
    let configs = parse_config_sections_cached(file_cache, "dbtable", constants);

    let mut server = PackedData::new(pack.max);

    for id in 0..pack.max {
        server.start_entry();

        let Some((debugname, props)) = config_entry("dbtable", &pack, &configs, id) else {
            server.finish_entry();
            continue;
        };

        let mut columns: Vec<ColumnDef> = Vec::new();

        // First pass: parse column definitions
        for (key, value) in props {
            if key == "column" {
                columns.push(parse_column_def(value));
            }
        }

        // Second pass: parse defaults
        for (key, value) in props {
            if key == "default" {
                if let Some((col_name, rest)) = value.split_once(',') {
                    let col_name = col_name.trim();
                    if let Some(col) = columns.iter_mut().find(|c| c.name == col_name) {
                        let vals: Vec<String> = parse_csv_values(rest);
                        col.defaults = vals;
                    }
                }
            }
        }

        if !columns.is_empty() {
            // Opcode 1: column definitions with types and defaults
            server.p1(1);
            server.p1(columns.len() as u8);
            for (col_idx, col) in columns.iter().enumerate() {
                let has_default = !col.defaults.is_empty();
                let flags = col_idx as u8 | if has_default { 0x80 } else { 0 };
                server.p1(flags);
                server.p1(col.types.len() as u8);
                for &t in &col.types {
                    server.p1(t);
                }
                if has_default {
                    server.p1(1); // 1 field set for defaults
                    for (i, val) in col.defaults.iter().enumerate() {
                        let type_char = col.types.get(i).copied().unwrap_or(b'i');
                        if type_char == b's' {
                            server.pjstr(val);
                        } else {
                            server.p4(resolve_typed_value(val, type_char, registry));
                        }
                    }
                }
            }
            server.p1(255); // end columns

            // Opcode 251: column names
            server.p1(251);
            server.p1(columns.len() as u8);
            for col in &columns {
                server.pjstr(&col.name);
            }

            // Opcode 252: column properties
            server.p1(252);
            server.p1(columns.len() as u8);
            for col in &columns {
                server.p1(col.props);
            }
        }

        server.p1(250);
        server.pjstr(debugname);
        server.finish_entry();
    }

    Ok(PackedFile {
        server,
        client: None,
    })
}

pub(super) fn parse_column_def(value: &str) -> ColumnDef {
    let mut parts = value.split(',').map(str::trim);
    let name = parts.next().unwrap().to_string();
    let mut types = Vec::new();
    let mut props = 0u8;

    for part in parts {
        match part {
            "INDEXED" => props |= PROP_INDEXED,
            "REQUIRED" => props |= PROP_REQUIRED,
            "LIST" => props |= PROP_LIST,
            "CLIENTSIDE" => props |= PROP_CLIENTSIDE,
            _ => {
                types.push(ScriptVarType::from_type_name(part) as u8);
            }
        }
    }

    ColumnDef {
        name,
        types,
        props,
        defaults: Vec::new(),
    }
}
