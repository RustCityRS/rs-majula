use std::collections::HashMap;

use crate::ParamValue;
use crate::pack::config::param::*;
use crate::pack::pack::{FileCache, parse_config_sections_cached};
use crate::pack::pack_registry::{PackFile, PackRegistry, PackedFile};
use crate::pack::packed_data::PackedData;
use crate::pack::util::parse_csv_values;
use crate::types::ScriptVarType;
use anyhow::{Result, ensure};
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
            #[cfg(before_647)]
            panic!("Unknown dbrow id: {}", id);
            #[cfg(since_647)]
            {
                server.finish_entry();
                continue;
            }
        };

        let Some(props) = configs.get(debugname) else {
            #[cfg(before_647)]
            panic!("Unknown dbrow config: {}", debugname);
            #[cfg(since_647)]
            {
                server.finish_entry();
                continue;
            }
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

        if !data_lines.is_empty() || cfg!(since_647) && schema.is_some() {
            let schema =
                schema.ok_or_else(|| anyhow::anyhow!("No table defined for dbrow: {debugname}"))?;

            #[cfg(since_647)]
            for (column, _) in &data_lines {
                ensure!(
                    schema.columns.iter().any(|(name, _, _)| name == column),
                    "{debugname}: unknown column {column}"
                );
            }

            // Opcode 3: column data - iterate in schema order like TypeScript
            server.p1(3);
            server.p1(schema.columns.len() as u8);

            for (col_idx, (col_name, col_types, col_props)) in schema.columns.iter().enumerate() {
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

                #[cfg(since_647)]
                if fields.is_empty() {
                    continue;
                }
                server.p1(col_idx as u8);
                server.p1(col_types.len() as u8);
                for &t in col_types {
                    server.p1(t);
                }

                server.p1(fields.len() as u8);
                for values in fields {
                    ensure!(
                        values.len() == col_types.len(),
                        "{debugname}: {col_name} has an incomplete tuple"
                    );
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
                        // A name that resolves to nothing packs as -1, which only fails
                        // at runtime ("Song not found: -1"): catch it here instead.
                        #[cfg(before_647)]
                        if matches!(value, ParamValue::Int(-1)) && val != "null" && val != "-1" {
                            return Err(anyhow::anyhow!(
                                "{debugname}: {col_name} has unknown {} [{val}]",
                                svt.type_name()
                            ));
                        }
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
                #[cfg(since_647)]
                let tid = u16::try_from(tid).expect("DB table reference exceeds 65535");
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

#[cfg(since_647)]
pub(crate) fn resolve_typed_value(value: &str, type_char: u8, registry: &PackRegistry) -> i32 {
    let ty = ScriptVarType::try_from(type_char).expect("Invalid config type");
    match get_param_value_for_type(ty.type_name(), registry, value) {
        Some(ParamValue::Int(value)) => value,
        _ => panic!("Expected integer value for {}: {value}", ty.type_name()),
    }
}

#[cfg(all(test, since_647))]
mod tests {
    use super::*;
    use crate::pack::pack::{collect_config_files, parse_config_sections};
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

    fn lookup_typed(value: &str, type_char: u8, registry: &PackRegistry) -> Option<i32> {
        let value = resolve_typed_value(value, type_char, registry);
        (value != -1).then_some(value)
    }

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
            let tc = crate::pack::config::param::type_name_to_char(ty) as u8;
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
            let tc = crate::pack::config::param::type_name_to_char(ty);
            assert!(
                LITERALS.contains(&ty)
                    || NAMESPACES
                        .iter()
                        .any(|(t, _)| { crate::pack::config::param::type_name_to_char(t) == tc }),
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
        assert_eq!(registry.obj.get_by_debugname("null"), Some(6657));
        assert_eq!(resolve_typed_value("null", b'o', &registry), -1);
        assert_eq!(resolve_typed_value("null", b'O', &registry), -1);
        // The cow sounds (synth 369, void-634's `cow_attack`), which 647
        // packed as -1 for want of a synth arm.
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
        let constants = crate::pack::pack::load_constants(&source);
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
                    .map(|p| crate::pack::config::param::type_name_to_char(p) as u8)
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
