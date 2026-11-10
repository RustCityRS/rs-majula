//! Spawned npcs that stand still at 647 say so in their server config.
//!
//! An npc with no `wanderrange` gets rs-pack's default of 5 and random-walks.
//! Two sources say which spawned npcs should not:
//! - cache 786's walk mode (npc opcode 119, `unknown119` in the unpacked
//!   configs): 1 marks an npc that does not move (void-634 reads it as its
//!   `ModeType.EMPTY`; Bob, the general stores' keepers, the combat dummies are
//!   all 1), and fishing spots (4 and 5) only relocate;
//! - 289's own configs, which give an npc its `wanderrange` (0 for Lumbridge's
//!   cook and Bob): a spawned npc 289 sets one for keeps one at 647.
//!
//! An authored `wanderrange` (any value, sourced in its config) satisfies both.
//!
//! ```text
//! cargo test -p rs-pack --test npc_wander_647
//! ```

#![cfg(since_647)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

fn files(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, ext, out);
        } else if path.extension().is_some_and(|e| e == ext) {
            out.push(path);
        }
    }
}

/// Every `[name]` section's keys across a tree's `.npc` configs.
fn npc_keys(dir: &Path) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut paths = Vec::new();
    files(dir, "npc", &mut paths);
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for path in paths {
        let mut current = None;
        for line in fs::read_to_string(&path).unwrap().lines() {
            let line = line.split(" //").next().unwrap().trim();
            if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                current = Some(name.to_string());
            } else if let (Some(name), Some((k, v))) = (&current, line.split_once('=')) {
                out.entry(name.clone())
                    .or_default()
                    .insert(k.to_string(), v.to_string());
            }
        }
    }
    out
}

/// The npc names the 647 map spawns (`maps/n<mx>_<mz>.jm2`: `level x z: name`).
fn spawned(root: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for entry in fs::read_dir(root.join("content/647/maps"))
        .unwrap()
        .flatten()
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if !(name.starts_with('n') && name.ends_with(".jm2")) {
            continue;
        }
        for line in fs::read_to_string(entry.path()).unwrap().lines() {
            if let Some((_, npc)) = line.split_once(':') {
                out.insert(npc.trim().to_string());
            }
        }
    }
    out
}

#[test]
fn npcs_the_cache_marks_still_do_not_wander() {
    let root = root();
    let npcs = npc_keys(&root.join("content/647/scripts"));
    let missing: Vec<_> = spawned(&root)
        .into_iter()
        .filter(|name| {
            let Some(keys) = npcs.get(name) else {
                return false;
            };
            let mode = keys.get("unknown119").map(String::as_str);
            let still = mode == Some("1")
                || (name.starts_with("fishing_spot") && matches!(mode, Some("4" | "5")));
            still && !keys.contains_key("wanderrange")
        })
        .collect();
    assert!(
        missing.is_empty(),
        "{} spawned npcs stand still in cache 786 but default to wanderrange 5: {:?}",
        missing.len(),
        missing
    );
}

#[test]
fn spawned_npcs_keep_289s_wanderrange() {
    let root = root();
    let npcs = npc_keys(&root.join("content/647/scripts"));
    let old = npc_keys(&root.join("content/289/scripts"));
    let lost: Vec<_> = spawned(&root)
        .into_iter()
        .filter(|name| {
            old.get(name).is_some_and(|k| k.contains_key("wanderrange"))
                && !npcs
                    .get(name)
                    .is_some_and(|k| k.contains_key("wanderrange"))
        })
        .collect();
    assert!(
        lost.is_empty(),
        "{} spawned npcs lost 289's wanderrange: {:?}",
        lost.len(),
        lost
    );
}

#[test]
fn lumbridge_keepers_and_dummies_stand_still() {
    let npcs = npc_keys(&root().join("content/647/scripts"));
    for name in [
        "bob",
        "cook",
        "generalshopkeeper1",
        "cws_doomsayer",
        "melee_dummy",
        "tutorial2_magic_dummy",
    ] {
        let range = npcs
            .get(name)
            .and_then(|k| k.get("wanderrange"))
            .map(String::as_str);
        assert_eq!(range, Some("0"), "{name}'s wanderrange");
    }
}
