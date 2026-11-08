//! Every `%var` a 647 script names is registered.
//!
//! Varps never auto-register, and neither do server vars. A name a `.varp` or
//! `.vars` config declares but the registry lacks still compiles -- the
//! compiler gives it an id past the registry's end -- and the first script to
//! touch it reads past the player's or world's var array and stops the engine.
//! Agility's course-progress varps did exactly that. This fails on any `%name`
//! in a script that no varp, varbit, var, varn or varc registry holds.
//!
//! ```text
//! cargo test -p rs-pack --test script_vars_registered_647
//! ```

#![cfg(since_647)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn names(pack: &Path) -> BTreeSet<String> {
    fs::read_to_string(pack)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once('=').map(|(_, n)| n.trim().to_string()))
        .collect()
}

fn scripts(dir: &Path, out: &mut Vec<PathBuf>) {
    out.extend(runec::sources::script_files(dir, "rs2").unwrap());
}

#[test]
fn every_script_var_is_registered() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let pack = root.join("content/647/pack");
    let mut registered = BTreeSet::new();
    for kind in ["varp", "varbit", "vars", "varn", "varc", "varcs"] {
        registered.extend(names(&pack.join(format!("{kind}.pack"))));
    }

    let mut files = Vec::new();
    scripts(&root.join("content/647/scripts/runescript"), &mut files);
    let mut missing = BTreeSet::new();
    for path in files {
        let text = fs::read_to_string(&path).unwrap();
        for (n, line) in text.lines().enumerate() {
            // Comments and string text are not code.
            let code = line.split("//").next().unwrap_or("");
            let mut in_string = false;
            let bytes = code.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                match bytes[i] {
                    b'"' => in_string = !in_string,
                    b'%' if !in_string => {
                        let start = i + 1;
                        let mut end = start;
                        while end < bytes.len()
                            && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                        {
                            end += 1;
                        }
                        let name = &code[start..end];
                        if name.starts_with(|c: char| c.is_ascii_lowercase())
                            && !registered.contains(name)
                        {
                            missing.insert(format!(
                                "{}:{}: %{name}",
                                path.strip_prefix(&root).unwrap_or(&path).display(),
                                n + 1
                            ));
                        }
                        i = end;
                        continue;
                    }
                    _ => {}
                }
                i += 1;
            }
        }
    }
    assert!(
        missing.is_empty(),
        "script vars no registry holds (they compile, then index past the var arrays):\n  {}",
        missing.into_iter().collect::<Vec<_>>().join("\n  ")
    );
}
