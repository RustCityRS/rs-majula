#![allow(clippy::unnecessary_unwrap)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! DbRow config packer — .dbrow text → dbrow.dat/dbrow.idx.
//! Server-only. Matches 2004scape `DbRowConfig.ts`.
//!
//! Row format:
//!   table=table_name
//!   data=column_name,value[,value]...

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use tracing::info;

use super::{collect_config_files, parse_config_sections};
use crate::pack_registry::PackRegistry;
use crate::packed_data::PackedData;

/// Loaded table schema for resolving column names and types.
struct TableSchema {
    columns: Vec<(String, Vec<u8>)>, // (name, type_chars)
}

pub fn pack_dbrows(
    source_dir: &Path,
    registry: &PackRegistry,
    constants: &HashMap<String, String>,
    output_dir: &Path,
) -> Result<()> {
    let row_pack = crate::pack_registry::PackFile::load(&registry.pack_dir().join("dbrow.pack"))?;
    let table_pack =
        crate::pack_registry::PackFile::load(&registry.pack_dir().join("dbtable.pack"))?;

    // Load table schemas so we know column types for each table
    let table_files = collect_config_files(source_dir, "dbtable");
    let table_configs = parse_config_sections(&table_files, constants);
    let mut table_schemas: HashMap<String, TableSchema> = HashMap::new();
    for (tname, props) in &table_configs {
        let mut columns = Vec::new();
        for (key, value) in props {
            if key == "column" {
                let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
                if !parts.is_empty() {
                    let col_name = parts[0].to_string();
                    let types: Vec<u8> = parts[1..]
                        .iter()
                        .filter(|p| !p.chars().all(|c| c.is_uppercase() || c == '_'))
                        .map(|p| crate::pack::param::type_name_to_char(p) as u8)
                        .collect();
                    columns.push((col_name, types));
                }
            }
        }
        table_schemas.insert(tname.clone(), TableSchema { columns });
    }

    let files = collect_config_files(source_dir, "dbrow");
    info!("  Found {} .dbrow files", files.len());
    let configs = parse_config_sections(&files, constants);
    info!("  Parsed {} dbrow configs", configs.len());

    let mut packed = PackedData::new(row_pack.max);

    for id in 0..row_pack.max {
        packed.start_entry();
        let Some(name) = row_pack.get_by_id(id) else {
            packed.finish_entry();
            continue;
        };
        let Some(props) = configs.get(name) else {
            packed.finish_entry();
            continue;
        };

        // Find table name
        let table_name = props
            .iter()
            .find(|(k, _)| k == "table")
            .map(|(_, v)| v.as_str());

        let schema = table_name.and_then(|tn| table_schemas.get(tn));

        // Opcode 4: table reference
        if let Some(tn) = table_name {
            if let Some(tid) = table_pack.get_by_name(tn) {
                packed.p1(4);
                packed.p2(tid as u16);
            }
        }

        // Collect data lines grouped by column
        let data_lines: Vec<(&str, Vec<String>)> = props
            .iter()
            .filter(|(k, _)| k == "data")
            .filter_map(|(_, v)| {
                let (col_name, rest) = v.split_once(',')?;
                let values = parse_csv_values(rest);
                Some((col_name.trim(), values))
            })
            .collect();

        if !data_lines.is_empty() && schema.is_some() {
            let schema = schema.unwrap();

            // Merge all `data=` lines that target the same column into a single
            // column with N tuples (matching 2004scape's dbrow wire format and
            // the runtime decoder, which looks up exactly one column per index).
            // A LIST column (e.g. `coord_pair`) is populated by repeating the
            // `data=` line — each line contributes one tuple, not its own column.
            // Emitting one wire-column per line instead made db_getfieldcount
            // return 1 (only the first column is found), so multi-row zone tables
            // like bank_zones / firemaking_zones only ever tested their first row.
            let mut columns: Vec<(usize, &[u8], Vec<String>)> = Vec::new();
            for (col_name, values) in &data_lines {
                let col_idx = schema
                    .columns
                    .iter()
                    .position(|(n, _)| n == col_name)
                    .unwrap_or(0);
                let col_types = schema
                    .columns
                    .get(col_idx)
                    .map(|(_, t)| t.as_slice())
                    .unwrap_or(b"i");
                match columns.iter_mut().find(|(idx, _, _)| *idx == col_idx) {
                    Some((_, _, acc)) => acc.extend(values.iter().cloned()),
                    None => columns.push((col_idx, col_types, values.clone())),
                }
            }

            // Opcode 3: column data
            packed.p1(3);
            packed.p1(columns.len() as u8);

            for (col_idx, col_types, values) in &columns {
                packed.p1(*col_idx as u8);
                packed.p1(col_types.len() as u8);
                for &t in *col_types {
                    packed.p1(t);
                }

                // Field (tuple) count: total values / per-tuple stride.
                let fields = if col_types.is_empty() {
                    0
                } else {
                    values.len() / col_types.len().max(1)
                };
                packed.p1(fields.max(1) as u8);

                for (i, val) in values.iter().enumerate() {
                    let type_char = col_types.get(i % col_types.len()).copied().unwrap_or(b'i');
                    if type_char == b's' {
                        packed.pjstr(val);
                    } else {
                        // Try resolving as entity name, fall back to int
                        let int_val = resolve_typed_value(val, type_char, registry);
                        packed.p4(int_val);
                    }
                }
            }
            packed.p1(255); // end columns
        }

        packed.p1(250);
        packed.pjstr(name);
        packed.finish_entry();
    }

    packed.save(output_dir, "dbrow")
}

/// The id a name has in the namespace of its type char (`o` obj, `l` loc,
/// `P` synth, `I` interface or `iface:comp` component, ...), or a coord
/// written `level_mx_mz_lx_lz`; `None` if it names nothing there or the type
/// has no namespace. Every typed-value resolver at 647 goes through this, so
/// each type resolves the same way in a param, dbrow or enum: a type missing
/// here packed as -1 whatever its name (every npc's sounds did).
pub(crate) fn lookup_typed(value: &str, type_char: u8, registry: &PackRegistry) -> Option<i32> {
    let id = match type_char {
        b'l' => registry.loc.get_by_name(value),
        b'o' | b'O' => registry.obj.get_by_name(value),
        b'n' => registry.npc.get_by_name(value),
        b'g' => registry.r#enum.get_by_name(value),
        b'A' => registry.seq.get_by_name(value),
        b'P' => registry.synth.get_by_name(value),
        b'y' => registry.category.get_by_name(value),
        b'v' => registry.inv.get_by_name(value),
        // stat / npc_stat (both map to 'S'): resolve the stat name to its id.
        // Without this a `stat`-typed dbrow field (e.g. consume_table.stat_heal)
        // packs as -1, which later crashes stat ops indexing levels[-1].
        b'S' => registry.stat.get_by_name(value),
        b'M' => registry.midi.get_by_name(value),
        b'J' => registry.r#struct.get_by_name(value),
        b't' => registry.spotanim.get_by_name(value),
        // 647 has no model registry: `model_<id>[_suffix]`, as the npc
        // packer reads model names.
        b'm' => registry.model.get_by_name(value).or_else(|| {
            value
                .strip_prefix("model_")
                .and_then(|rest| rest.split('_').next())
                .and_then(|id| id.parse().ok())
        }),
        b'V' => registry.varp.get_by_name(value),
        b'K' => registry.idk.get_by_name(value),
        // `iface:comp` is a component's hash; a bare name, an interface id.
        b'I' if value.contains(':') => {
            return registry.component_hashes.get(value).copied();
        }
        b'I' => registry.interface.get_by_name(value),
        0xEF => {
            // dbrow (ï = 0xEF)
            let dbrow_pack =
                crate::pack_registry::PackFile::load(&registry.pack_dir().join("dbrow.pack")).ok();
            dbrow_pack.and_then(|p| p.get_by_name(value))
        }
        b'c' => {
            return match parse_coord_value(value) {
                -1 => None,
                coord => Some(coord),
            };
        }
        _ => None,
    };
    id.map(|id| id as i32)
}

/// An int-typed config value as a number: a literal, a boolean (`true`/`yes`
/// 1, `false`/`no` 0), or the id of the symbol it names in the namespace of
/// its type ([`lookup_typed`]). An unresolved name is -1, as in Jordan's
/// packer.
pub(crate) fn resolve_typed_value(value: &str, type_char: u8, registry: &PackRegistry) -> i32 {
    if let Ok(v) = value.parse::<i32>() {
        return v;
    }
    // `null` is -1 in every type, as params and enums already read it: 2011
    // names obj 6657 "null", so looked up as a name it packed 289's
    // `data=convertobj,null,banana,...` as that obj.
    if value == "null" {
        return -1;
    }
    // A boolean column's `true` packed as -1, so 289's `data=gate,true` read
    // as false and its `data=members,false` as not false either.
    if type_char == b'1' {
        match value {
            "true" | "yes" => return 1,
            "false" | "no" => return 0,
            _ => {}
        }
    }
    lookup_typed(value, type_char, registry).unwrap_or(-1)
}

/// A coord written `level_mx_mz_lx_lz`, packed `level << 28 | x << 14 | z`;
/// -1 if it is not five parts.
pub(crate) fn parse_coord_value(s: &str) -> i32 {
    let parts: Vec<&str> = s.split('_').collect();
    if parts.len() != 5 {
        return -1;
    }
    let level: u32 = parts[0].parse().unwrap_or(0);
    let mx: u32 = parts[1].parse().unwrap_or(0);
    let mz: u32 = parts[2].parse().unwrap_or(0);
    let lx: u32 = parts[3].parse().unwrap_or(0);
    let lz: u32 = parts[4].parse().unwrap_or(0);
    let x = mx * 64 + lx;
    let z = mz * 64 + lz;
    ((level & 0x3) << 28 | (x & 0x3FFF) << 14 | (z & 0x3FFF)) as i32
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    /// Every config type that names something, with its 647 pack registry.
    /// A type here with no `lookup_typed` arm packed as -1 whatever its name.
    const NAMESPACES: &[(&str, &str)] = &[
        ("obj", "obj"),
        ("namedobj", "obj"),
        ("npc", "npc"),
        ("loc", "loc"),
        ("enum", "enum"),
        ("struct", "struct"),
        ("stat", "stat"),
        ("npc_stat", "stat"),
        ("seq", "seq"),
        ("synth", "synth"),
        ("midi", "midi"),
        ("inv", "inv"),
        ("spotanim", "spotanim"),
        ("varp", "varp"),
        ("category", "category"),
        ("idkit", "idk"),
        ("interface", "interface"),
        ("dbrow", "dbrow"),
    ];

    /// Types with no namespace: literals, runtime values a config never names,
    /// and models (`model_<id>` names, checked below).
    const LITERALS: &[&str] = &[
        "model",
        "int",
        "string",
        "boolean",
        "coord",
        "mapelement",
        "player_uid",
        "npc_uid",
    ];

    fn registry() -> PackRegistry {
        PackRegistry::load(&Path::new("..").join(crate::PACK_DIR)).expect("load registry")
    }

    #[test]
    fn every_namespace_resolves_its_own_names() {
        let registry = registry();
        for (ty, pack) in NAMESPACES {
            let text = std::fs::read_to_string(registry.pack_dir().join(format!("{pack}.pack")))
                .expect(pack);
            let (id, name) = text
                .lines()
                .filter_map(|l| l.split_once('='))
                .filter_map(|(id, n)| Some((id.parse::<i32>().ok()?, n)))
                .next_back()
                .unwrap_or_else(|| panic!("{pack}.pack names nothing"));
            let tc = crate::pack::param::type_name_to_char(ty) as u8;
            assert_eq!(
                lookup_typed(name, tc, &registry),
                Some(id),
                "a {ty} names {name} ({id}) in {pack}.pack"
            );
        }
    }

    /// Each type a 647 param can declare is a namespace above or a literal,
    /// so a new type cannot fall through to -1 unnoticed.
    #[test]
    fn every_param_type_is_a_namespace_or_a_literal() {
        let types = [
            "int",
            "string",
            "coord",
            "obj",
            "namedobj",
            "npc",
            "loc",
            "component",
            "interface",
            "boolean",
            "enum",
            "struct",
            "stat",
            "npc_stat",
            "seq",
            "synth",
            "midi",
            "inv",
            "spotanim",
            "varp",
            "model",
            "mapelement",
            "category",
            "idkit",
            "player_uid",
            "npc_uid",
            "dbrow",
        ];
        for ty in types {
            let tc = crate::pack::param::type_name_to_char(ty);
            assert!(
                LITERALS.contains(&ty)
                    || NAMESPACES
                        .iter()
                        .any(|(t, _)| { crate::pack::param::type_name_to_char(t) == tc }),
                "type {ty} ('{tc}') is neither a namespace nor a literal"
            );
        }
    }

    #[test]
    fn components_coords_and_sounds_resolve() {
        let registry = registry();
        assert_eq!(
            lookup_typed("chatbox:com_dialogue_box_small", b'I', &registry),
            registry
                .component_hashes
                .get("chatbox:com_dialogue_box_small")
                .copied()
        );
        assert!(
            registry
                .component_hashes
                .contains_key("chatbox:com_dialogue_box_small")
        );
        assert_eq!(
            lookup_typed("0_48_155_48_44", b'c', &registry),
            Some((48 * 64 + 48) << 14 | (155 * 64 + 44))
        );
        // Models have no registry at 647, only `model_<id>` names.
        assert_eq!(
            lookup_typed("model_23889_npc", b'm', &registry),
            Some(23889)
        );
        // `null` is no obj, though 2011 names obj 6657 "null".
        assert_eq!(lookup_typed("null", b'o', &registry), Some(6657));
        assert_eq!(resolve_typed_value("null", b'o', &registry), -1);
        assert_eq!(resolve_typed_value("null", b'O', &registry), -1);
        // 2011's cow sounds, which 647 packed as -1 for want of a synth arm.
        assert_eq!(resolve_typed_value("cow_attack", b'P', &registry), 369);
        assert_eq!(resolve_typed_value("no_such_sound", b'P', &registry), -1);
    }

    /// Every value a 647 dbrow gives a typed column names something there,
    /// or is a literal, a boolean or `null`; or it is a known gap. The packer
    /// writes a value that names nothing as -1, without a word: 289's
    /// `data=stat,runecraft` (2011's stat is `runecrafting`) left 2011's
    /// Runecrafting level-up without its box, and every boolean (`data=gate,
    /// true`) read as -1 until the packer learned them. Fails on a value
    /// `tests/dbrow_refs_647.known.tsv` does not list, and on a listed one
    /// that now resolves, so the list only shrinks.
    #[test]
    fn every_dbrow_value_names_something() {
        use std::collections::BTreeSet;
        let registry = registry();
        let source = Path::new("..").join(crate::CONTENT_DIR).join("scripts");
        let constants = super::super::load_constants(&source);
        let tables = parse_config_sections(&collect_config_files(&source, "dbtable"), &constants);
        let mut schemas: HashMap<String, HashMap<String, Vec<u8>>> = HashMap::new();
        for (table, props) in &tables {
            let columns = schemas.entry(table.clone()).or_default();
            for (key, value) in props {
                if key != "column" {
                    continue;
                }
                let parts: Vec<&str> = value.split(',').map(str::trim).collect();
                let types = parts[1..]
                    .iter()
                    .filter(|p| !p.chars().all(|c| c.is_uppercase() || c == '_'))
                    .map(|p| crate::pack::param::type_name_to_char(p) as u8)
                    .collect();
                columns.insert(parts[0].to_string(), types);
            }
        }
        let rows = parse_config_sections(&collect_config_files(&source, "dbrow"), &constants);
        let mut found = BTreeSet::new();
        for (row, props) in &rows {
            let Some(columns) = props
                .iter()
                .find(|(k, _)| k == "table")
                .and_then(|(_, t)| schemas.get(t))
            else {
                continue;
            };
            for (key, value) in props {
                let Some((column, rest)) = value.split_once(',').filter(|_| key == "data") else {
                    continue;
                };
                let Some(types) = columns.get(column.trim()).filter(|t| !t.is_empty()) else {
                    continue;
                };
                for (i, v) in parse_csv_values(rest).iter().enumerate() {
                    let tc = types[i % types.len()];
                    let literal = tc == b's'
                        || v == "null"
                        || v.parse::<i32>().is_ok()
                        || (tc == b'1' && ["true", "false", "yes", "no"].contains(&v.as_str()));
                    if !literal && lookup_typed(v, tc, &registry).is_none() {
                        found.insert(format!("{row}\t{}\t{v}", column.trim()));
                    }
                }
            }
        }
        let known_path = Path::new("tests/dbrow_refs_647.known.tsv");
        let known: BTreeSet<String> = std::fs::read_to_string(known_path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(str::to_string)
            .collect();
        let new: Vec<_> = found.difference(&known).collect();
        let fixed: Vec<_> = known.difference(&found).collect();
        assert!(
            new.is_empty() && fixed.is_empty(),
            "dbrow values naming nothing, not in {}: {new:#?}
listed but now resolving: {fixed:#?}",
            known_path.display()
        );
    }
}
