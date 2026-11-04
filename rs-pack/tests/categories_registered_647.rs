//! Every category an authored 647 config names is in `category.pack`.
//!
//! The obj, loc and npc packers look a `category=` value up in the registry
//! and, finding nothing, write no category at all -- no warning. The compiler
//! still resolves `_name` triggers and `oc_category(..) = name` against the
//! name, so the script builds and its trigger never fires: an unregistered
//! `potion` left every potion uncombinable. Categories never auto-register,
//! so a port adds its own; this fails on any it forgot.
//!
//! ```text
//! cargo test -p rs-pack --test categories_registered_647
//! ```

#![cfg(since_647)]

use std::collections::BTreeSet;
use std::path::Path;

fn configs(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "_unpack") {
                configs(&path, out);
            }
        } else if path
            .extension()
            .is_some_and(|e| e == "obj" || e == "loc" || e == "npc")
        {
            out.push(path);
        }
    }
}

#[test]
fn every_config_category_is_registered() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let registered: BTreeSet<String> =
        std::fs::read_to_string(root.join("content/647/pack/category.pack"))
            .unwrap()
            .lines()
            .filter_map(|l| l.split_once('=').map(|(_, n)| n.trim().to_string()))
            .collect();

    let mut files = Vec::new();
    configs(&root.join("content/647/scripts"), &mut files);
    assert!(files.len() > 10, "found only {} configs", files.len());

    let mut missing = Vec::new();
    for path in files {
        let text = std::fs::read_to_string(&path).unwrap();
        for (n, line) in text.lines().enumerate() {
            if let Some(name) = line.trim().strip_prefix("category=") {
                if !registered.contains(name) {
                    missing.push(format!(
                        "{}:{}: `{name}`",
                        path.strip_prefix(&root).unwrap_or(&path).display(),
                        n + 1
                    ));
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "categories not in content/647/pack/category.pack (the packer drops them):\n  {}",
        missing.join("\n  ")
    );
}
