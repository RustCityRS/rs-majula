//! Every timer and queue a 647 script names is a script of that kind.
//!
//! Outside `#testscript` files the compiler accepts a name of another kind
//! where a timer or queue belongs. 289's `cleartimer(prayer_drain)` typechecked
//! against the synth `prayer_drain` while the timer was unported, and compiled
//! to -1 (rs-compiler 0.4.0's fallback for a script it cannot find); the clear
//! failed at runtime and ended the death queue before the respawn, with no
//! error. This fails on any
//! command whose timer or queue argument names no `[timer,..]`,
//! `[softtimer,..]` or queue script in 647's content.
//!
//! ```text
//! cargo test -p rs-pack --test script_refs_defined_647
//! ```

#![cfg(since_647)]

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn scripts(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            scripts(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs2") {
            out.push(path);
        }
    }
}

/// The script kinds a reference of `ty` may name.
fn kinds(ty: &str) -> &'static [&'static str] {
    match ty {
        "timer" => &["timer"],
        "softtimer" => &["softtimer"],
        _ => &["queue", "weakqueue", "strongqueue"],
    }
}

/// Commands whose first parameter is a timer or queue, from `engine.rs2`,
/// including the vararg ones whose signature is a comment.
fn ref_commands(engine: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in engine.lines() {
        let Some(rest) = line.strip_prefix("[command,") else {
            continue;
        };
        let Some((name, sig)) = rest.split_once(']') else {
            continue;
        };
        let sig = sig.trim_start_matches("/*").trim_start();
        let Some(first) = sig.strip_prefix('(') else {
            continue;
        };
        let ty = first.split_whitespace().next().unwrap_or("");
        if matches!(ty, "timer" | "softtimer" | "queue") {
            out.insert(name.trim_end_matches('*').to_string(), ty.to_string());
        }
    }
    out
}

/// A line with its comments and string text blanked, so neither is read as
/// code.
fn code(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_string {
            if c == '"' {
                in_string = false;
                out.push('"');
            } else {
                out.push(' ');
            }
        } else if c == '"' {
            in_string = true;
            out.push('"');
        } else if line[i..].starts_with("//") {
            break;
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

fn ident_at(s: &str) -> Option<&str> {
    let end = s
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(s.len());
    (end > 0 && s.as_bytes()[0].is_ascii_lowercase()).then(|| &s[..end])
}

#[test]
fn every_named_timer_and_queue_is_defined() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content/647/scripts");
    let commands = ref_commands(&std::fs::read_to_string(root.join("engine.rs2")).unwrap());
    assert!(
        commands.contains_key("cleartimer") && commands.contains_key("queue"),
        "read no timer or queue commands from engine.rs2: {commands:?}"
    );

    let mut files = Vec::new();
    scripts(&root, &mut files);
    let mut defined = BTreeSet::new();
    for path in &files {
        for line in std::fs::read_to_string(path).unwrap().lines() {
            if let Some((kind, name)) = line
                .strip_prefix('[')
                .and_then(|l| l.split_once(']'))
                .and_then(|(head, _)| head.split_once(','))
            {
                defined.insert((kind.to_string(), name.to_string()));
            }
        }
    }

    let mut missing = BTreeSet::new();
    for path in &files {
        let text = std::fs::read_to_string(path).unwrap();
        for (n, line) in text.lines().enumerate() {
            if line.starts_with('[') {
                continue;
            }
            let line = code(line);
            for (cmd, ty) in &commands {
                let mut from = 0;
                while let Some(at) = line[from..].find(cmd.as_str()) {
                    let start = from + at;
                    from = start + cmd.len();
                    let before = line[..start].chars().next_back();
                    if before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                        continue;
                    }
                    let after = line[from..].trim_start_matches('*');
                    let Some(arg) = after.strip_prefix('(').map(str::trim_start) else {
                        continue;
                    };
                    let Some(name) = ident_at(arg) else {
                        continue;
                    };
                    if !kinds(ty)
                        .iter()
                        .any(|k| defined.contains(&(k.to_string(), name.to_string())))
                    {
                        missing.insert(format!(
                            "{}:{}: {cmd}({name}) names no {ty} script",
                            path.strip_prefix(&root).unwrap_or(path).display(),
                            n + 1
                        ));
                    }
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "timers and queues no script defines (they compile to another kind's id):\n  {}",
        missing.into_iter().collect::<Vec<_>>().join("\n  ")
    );
}
