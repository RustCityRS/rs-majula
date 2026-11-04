//! No varp past the client's own transmits.
//!
//! 647's client sizes its varp array by the cache's varps (`_unpack/all.varp`,
//! 2,181 of them). Content may add server-only varps after those, but a
//! transmitted one reaches the client as an index it does not have: 289's
//! `dueloptions` (2190) kept its `transmit=yes`, the login varp sync sent it,
//! and the client threw `ArrayIndexOutOfBoundsException: Index 2190 out of
//! bounds for length 2181` and dropped every login. This fails on any varp at
//! or past the client's count that a `.varp` config marks `transmit=yes`.
//!
//! ```text
//! cargo test -p rs-pack --test server_only_varps_never_transmit_647
//! ```

#![cfg(since_647)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn configs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != "_unpack") {
                configs(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "varp") {
            out.push(path);
        }
    }
}

#[test]
fn no_server_only_varp_transmits() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647");
    let ids: BTreeMap<String, u32> = std::fs::read_to_string(root.join("pack/varp.pack"))
        .unwrap()
        .lines()
        .filter_map(|l| l.split_once('='))
        .filter_map(|(id, name)| Some((name.to_string(), id.parse().ok()?)))
        .collect();
    // The client's count: one past the highest cache varp.
    let client_count = std::fs::read_to_string(root.join("scripts/_unpack/all.varp"))
        .expect("run `cargo unpack` first")
        .lines()
        .filter_map(|l| l.strip_prefix('[')?.strip_suffix(']'))
        .filter_map(|name| ids.get(name))
        .max()
        .map(|m| m + 1)
        .expect("all.varp names no registered varp");

    let mut files = Vec::new();
    configs(&root.join("scripts"), &mut files);
    let mut bad = Vec::new();
    for path in files {
        let mut section = String::new();
        for (n, line) in std::fs::read_to_string(&path).unwrap().lines().enumerate() {
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.to_string();
            } else if line.trim() == "transmit=yes"
                && let Some(&id) = ids.get(&section)
                && id >= client_count
            {
                bad.push(format!(
                    "{}:{}: [{section}] is varp {id}, past the client's {client_count}",
                    path.strip_prefix(&root).unwrap_or(&path).display(),
                    n + 1
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "server-only varps marked transmit=yes (the client drops the login):\n  {}",
        bad.join("\n  ")
    );
}
