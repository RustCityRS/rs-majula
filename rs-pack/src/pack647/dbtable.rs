//! DbTable config packer — .dbtable text → dbtable.dat/dbtable.idx.
//! Server-only. Matches 2004scape `DbTableConfig.ts`.
//!
//! Column format: `column=name,type[,type]...[,INDEXED][,REQUIRED][,LIST][,CLIENTSIDE]`
//! Default format: `default=column_name,value[,value]...`

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

const PROP_INDEXED: u8 = 0x1;
const PROP_REQUIRED: u8 = 0x2;
const PROP_LIST: u8 = 0x4;
const PROP_CLIENTSIDE: u8 = 0x8;

struct ColumnDef {
    name: String,
    types: Vec<u8>,
    props: u8,
    defaults: Vec<String>,
}

pub fn pack_dbtables(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    output_dir: &Path,
) -> Result<()> {
    let pack = crate::pack_registry::PackFile::load(&registry.pack_dir().join("dbtable.pack"))?;
    let files = collect_config_files(source_dir, "dbtable");
    info!("  Found {} .dbtable files", files.len());
    let configs = parse_config_sections(&files, constants);
    info!("  Parsed {} dbtable configs", configs.len());

    let mut packed = PackedData::new(pack.max);

    for id in 0..pack.max {
        packed.start_entry();
        let Some(name) = pack.get_by_id(id) else {
            packed.finish_entry();
            continue;
        };
        let Some(props) = configs.get(name) else {
            packed.finish_entry();
            continue;
        };

        let mut columns: Vec<ColumnDef> = Vec::new();

        // First pass: parse column definitions
        for (key, value) in props {
            if key == "column" {
                if let Some(col) = parse_column_def(value) {
                    columns.push(col);
                }
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
            packed.p1(1);
            packed.p1(columns.len() as u8);
            for (col_idx, col) in columns.iter().enumerate() {
                let has_default = !col.defaults.is_empty();
                let flags = col_idx as u8 | if has_default { 0x80 } else { 0 };
                packed.p1(flags);
                packed.p1(col.types.len() as u8);
                for &t in &col.types {
                    packed.p1(t);
                }
                if has_default {
                    packed.p1(1); // 1 field set for defaults
                    for (i, val) in col.defaults.iter().enumerate() {
                        let type_char = col.types.get(i).copied().unwrap_or(b'i');
                        if type_char == b's' {
                            packed.pjstr(val);
                        } else {
                            packed.p4(super::dbrow::resolve_typed_value(val, type_char, registry));
                        }
                    }
                }
            }
            packed.p1(255); // end columns

            // Opcode 251: column names
            packed.p1(251);
            packed.p1(columns.len() as u8);
            for col in &columns {
                packed.pjstr(&col.name);
            }

            // Opcode 252: column properties
            packed.p1(252);
            packed.p1(columns.len() as u8);
            for col in &columns {
                packed.p1(col.props);
            }
        }

        packed.p1(250);
        packed.pjstr(name);
        packed.finish_entry();
    }

    packed.save(output_dir, "dbtable")
}

fn parse_column_def(value: &str) -> Option<ColumnDef> {
    let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
    if parts.is_empty() {
        return None;
    }

    let name = parts[0].to_string();
    let mut types = Vec::new();
    let mut props = 0u8;

    for &part in &parts[1..] {
        match part {
            "INDEXED" => props |= PROP_INDEXED,
            "REQUIRED" => props |= PROP_REQUIRED,
            "LIST" => props |= PROP_LIST,
            "CLIENTSIDE" => props |= PROP_CLIENTSIDE,
            _ => {
                let type_char = crate::pack::param::type_name_to_char(part);
                types.push(type_char as u8);
            }
        }
    }

    Some(ColumnDef {
        name,
        types,
        props,
        defaults: Vec::new(),
    })
}

fn parse_csv_values(s: &str) -> Vec<String> {
    let mut vals = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in s.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if ch == ',' && !in_quotes {
            vals.push(current.trim().to_string());
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    vals.push(current.trim().to_string());
    vals
}
