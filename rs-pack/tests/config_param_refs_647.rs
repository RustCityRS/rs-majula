//! Every named param value in a 647 config names something, or is a known gap.
//!
//! The config packer (`get_param_value_for_type`) packs a value that names
//! nothing in its registry as -1, without a word: 289's
//! `param=attack_anim,cow_attack` packed so while 647's registry had no
//! `cow_attack`, and the field cow never animated. This reads every
//! `param=<name>,<value>` in 647's npc, obj, loc and struct configs whose
//! param type names a registry (seq, synth, obj, npc, ...) and fails on a
//! value that registry lacks, unless `config_param_refs_647.known.tsv` lists
//! it. It also fails on a listed pair that now resolves, so the list only
//! shrinks.
//!
//! ```text
//! cargo test -p rs-pack --test config_param_refs_647
//! ```

#![cfg(since_647)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

const KNOWN: &str = "tests/config_param_refs_647.known.tsv";

fn files(dir: &Path, exts: &[&str], out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, exts, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| exts.contains(&e))
        {
            out.push(path);
        }
    }
}

/// The pack registry a param type's values name.
fn registry(ty: &str) -> Option<&'static str> {
    Some(match ty {
        "obj" | "namedobj" => "obj",
        "npc" => "npc",
        "loc" => "loc",
        "enum" => "enum",
        "struct" => "struct",
        "stat" => "stat",
        "npc_stat" => "npc_stat",
        "seq" => "seq",
        "synth" => "synth",
        "inv" => "inv",
        "spotanim" => "spotanim",
        "varp" => "varp",
        "category" => "category",
        "idkit" => "idk",
        "interface" => "interface",
        _ => return None,
    })
}

/// A section header's name, from a `[name]` line.
fn header(line: &str) -> Option<&str> {
    line.strip_prefix('[')?.strip_suffix(']')
}

#[test]
fn every_config_param_value_names_something() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647");

    let mut param_files = Vec::new();
    files(&root.join("scripts"), &["param"], &mut param_files);
    let mut types = HashMap::new();
    for path in &param_files {
        let mut name = None;
        for line in std::fs::read_to_string(path).unwrap().lines() {
            let line = line.trim();
            if let Some(h) = header(line) {
                name = Some(h.to_string());
                types
                    .entry(h.to_string())
                    .or_insert_with(|| "int".to_string());
            } else if let (Some(ty), Some(n)) = (line.strip_prefix("type="), &name) {
                types.insert(n.clone(), ty.to_string());
            }
        }
    }
    assert_eq!(types.get("attack_anim").map(String::as_str), Some("seq"));

    let mut names: HashMap<&str, HashSet<String>> = HashMap::new();
    let mut unresolved: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
    let mut configs = Vec::new();
    files(
        &root.join("scripts"),
        &["npc", "obj", "loc", "struct"],
        &mut configs,
    );
    for path in &configs {
        let file = path
            .strip_prefix(root.join("scripts"))
            .unwrap()
            .display()
            .to_string()
            .replace('\\', "/");
        for line in std::fs::read_to_string(path).unwrap().lines() {
            let Some((param, value)) = line
                .trim()
                .strip_prefix("param=")
                .and_then(|l| l.split_once(','))
            else {
                continue;
            };
            let value = value.trim();
            let Some(kind) = types.get(param).and_then(|t| registry(t)) else {
                continue;
            };
            if value.starts_with('^') || value == "null" || value.parse::<i64>().is_ok() {
                continue;
            }
            let known = names.entry(kind).or_insert_with(|| {
                std::fs::read_to_string(root.join(format!("pack/{kind}.pack")))
                    .unwrap()
                    .lines()
                    .filter_map(|l| l.split_once('=').map(|(_, n)| n.to_string()))
                    .collect()
            });
            if !known.contains(value) {
                unresolved
                    .entry((param.to_string(), value.to_string()))
                    .or_default()
                    .insert(file.clone());
            }
        }
    }

    let listed: BTreeSet<(String, String)> =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(KNOWN))
            .unwrap()
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(|l| {
                let mut cols = l.split('\t');
                (
                    cols.next().unwrap().to_string(),
                    cols.next().unwrap_or("").to_string(),
                )
            })
            .collect();
    let new: Vec<String> = unresolved
        .iter()
        .filter(|(k, _)| !listed.contains(*k))
        .map(|((p, v), fs)| {
            format!(
                "param={p},{v} ({})",
                fs.iter().cloned().collect::<Vec<_>>().join(", ")
            )
        })
        .collect();
    let fixed: Vec<String> = listed
        .iter()
        .filter(|k| !unresolved.contains_key(*k))
        .map(|(p, v)| format!("{p}\t{v}"))
        .collect();
    assert!(
        new.is_empty(),
        "{} config param values name nothing in their registry (they pack as -1):\n  {}",
        new.len(),
        new.join("\n  ")
    );
    assert!(
        fixed.is_empty(),
        "these now resolve; remove them from {KNOWN}:\n  {}",
        fixed.join("\n  ")
    );
}
