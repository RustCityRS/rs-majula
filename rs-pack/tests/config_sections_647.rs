//! Every npc, obj and loc definition names a registered entity or a documented gap.
#![cfg(since_647)]

use rs_pack::pack::pack::collect_config_files;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const KNOWN: &str = "tests/config_sections_647.known.tsv";
const KINDS: [&str; 3] = ["npc", "obj", "loc"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rs-pack sits directly under the repo root")
        .to_path_buf()
}

fn registry(kind: &str) -> BTreeSet<String> {
    let path = repo_root().join(format!("content/647/pack/{kind}.pack"));
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter(|(id, _)| id.parse::<u32>().is_ok())
        .map(|(_, name)| name.to_string())
        .collect()
}

#[test]
fn every_config_section_names_something() {
    let root = repo_root();
    // (kind, name) -> the file defining it.
    let mut dead: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut checked = 0;
    for kind in KINDS {
        let names = registry(kind);
        for path in collect_config_files(&root.join("content/647/scripts"), kind) {
            let rel = path
                .strip_prefix(&root)
                .unwrap_or(path.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {rel}: {e}"));
            for line in text.lines() {
                let Some(name) = line.strip_prefix('[').and_then(|r| r.strip_suffix(']')) else {
                    continue;
                };
                checked += 1;
                if !names.contains(name) {
                    dead.insert((kind.to_string(), name.to_string()), rel.clone());
                }
            }
        }
    }
    assert!(
        checked > 1000,
        "only {checked} sections checked; the walk is broken"
    );

    let listed: BTreeSet<(String, String)> =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(KNOWN))
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
    let new: Vec<String> = dead
        .iter()
        .filter(|(k, _)| !listed.contains(*k))
        .map(|((kind, name), file)| format!("{kind}\t{name}\t{file}"))
        .collect();
    let fixed: Vec<String> = listed
        .iter()
        .filter(|k| !dead.contains_key(*k))
        .map(|(kind, name)| format!("{kind}\t{name}"))
        .collect();
    assert!(
        new.is_empty(),
        "{} config sections name nothing at 647 (they pack nothing):\n  {}",
        new.len(),
        new.join("\n  ")
    );
    assert!(
        fixed.is_empty(),
        "these now name something; remove them from {KNOWN}:\n  {}",
        fixed.join("\n  ")
    );
}
