//! Every trigger header in `content/647` names a subject 647 has.
//!
//! `[ai_queue3,goblin_greenarmour]` compiles whether or not 647 has an npc of
//! that name: the subject resolves to nothing and the block can never fire.
//! 289's drop tables carried three such headers into 647 -- 289's armoured
//! village goblins and Entrana's fire bird are other npcs at 2011 -- and a
//! 647-era flavour script a fourth (289's `sheepsheered`, 2011's `sheep`).
//! `script_trigger_headers_647.rs` checks a header's trigger word; this checks
//! its subject.
//!
//! An npc trigger (`ai_*`, `opnpc*`, `apnpc*`) must name an npc in
//! `pack/npc.pack`, an obj trigger (`opobj*`, `apobj*`, `opheld*`) an obj, a
//! loc trigger (`oploc*`, `aploc*`) a loc; `_<name>` must be a category in
//! `pack/category.pack`, and `_` alone matches every subject. The targeted
//! forms (`opnpct`, `apnpct`, ...) name the component a spell or item was
//! used from, not an npc, and are left out.
//!
//! ```text
//! cargo test -p rs-pack --test script_trigger_subjects_647
//! ```

#![cfg(since_647)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rs-pack sits directly under the repo root")
        .to_path_buf()
}

/// The names in a `content/647/pack/<kind>.pack` registry.
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

/// Every server script under `content/647/scripts`, tests and the unpacked tree left
/// out.
fn script_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
            let path = entry.expect("dir entry").path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.is_dir() {
                if name != "_unpack" && name != "_test" {
                    walk(&path, out);
                }
            } else if path
                .extension()
                .is_some_and(|ext| ext == "rs2" || ext == "gs2")
            {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&repo_root().join("content/647/scripts"), &mut out);
    assert!(
        out.len() > 50,
        "only {} .rs2 files found; the walk is broken",
        out.len()
    );
    out
}

/// The registry an untargeted trigger's subject belongs to, if it names one.
fn subject_kind(trigger: &str) -> Option<&'static str> {
    let targeted = trigger.ends_with('t') || trigger.ends_with('u');
    let starts = |p: &str| trigger.starts_with(p);
    if starts("ai_") {
        Some("npc")
    } else if targeted {
        None
    } else if starts("opnpc") || starts("apnpc") {
        Some("npc")
    } else if starts("opobj") || starts("apobj") || starts("opheld") {
        Some("obj")
    } else if starts("oploc") || starts("aploc") {
        Some("loc")
    } else {
        None
    }
}

#[test]
fn every_trigger_subject_names_something() {
    let npc = registry("npc");
    let obj = registry("obj");
    let loc = registry("loc");
    let category = registry("category");
    let root = repo_root();
    let mut checked = 0;
    let mut unknown = Vec::new();
    for path in script_files() {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path.as_path())
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        for (index, line) in text.lines().enumerate() {
            let Some((trigger, subject)) = line
                .strip_prefix('[')
                .and_then(|r| r.split_once(']'))
                .and_then(|(head, _)| head.split_once(','))
            else {
                continue;
            };
            let Some(kind) = subject_kind(trigger) else {
                continue;
            };
            checked += 1;
            let known = match subject.strip_prefix('_') {
                Some("") => true,
                Some(cat) => category.contains(cat),
                None => match kind {
                    "npc" => npc.contains(subject),
                    "obj" => obj.contains(subject),
                    _ => loc.contains(subject),
                },
            };
            if !known {
                unknown.push(format!("{rel}:{}: [{trigger},{subject}]", index + 1));
            }
        }
    }
    assert!(
        checked > 500,
        "only {checked} subjects checked; the parse is broken"
    );
    assert!(
        unknown.is_empty(),
        "trigger subjects 647 has no npc, obj, loc or category for (they can never fire):\n  {}",
        unknown.join("\n  ")
    );
}
