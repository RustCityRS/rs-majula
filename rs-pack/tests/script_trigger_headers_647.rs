//! Every trigger header in `content/647` names a trigger the engine can fire.
//!
//! A RuneScript file is a list of `[<trigger>,<subject>]` blocks. The trigger
//! word is not free text: `rs-runec` turns it into the `lookup` key stored in
//! the compiled script, and the engine finds scripts by building that same key
//! out of [`ServerTriggerType`](../../rs-engine/rs-vm/src/trigger.rs). A word
//! the compiler does not know still compiles -- it becomes a script with
//! `lookup = -1`, which `ScriptProvider` never inserts into `lookups` -- so the
//! block is accepted, packed, given an id, and can never run.
//!
//! `switch_window_mode.rs2` carried exactly that: a `[switch_window_mode,_]`
//! header whose body called `~switch_window_mode()`. Reading the compiled
//! `script.dat` at the offset of that name shows the four lookup bytes as
//! `ff ff ff ff`, the same -1 a `[proc,..]` gets, so nothing could ever fire it;
//! the mode change is driven from `rs-engine/src/handlers/window_status.rs`,
//! which runs `[proc,switch_window_mode]` by name. The header is removed and
//! this file is the guard against another one being written.
//!
//! **Normalisation.** Headers are snake_case (`ai_queue5`, `inv_buttond`) and
//! the enum is CamelCase (`AiQueue5`, `InvButtonD`); dropping `_` and
//! lower-casing lines them up, with one documented exception. Rev-647 merged
//! the old OpHeld/InvButton client packets into `IfButton1-10`, so content
//! writes `[if_button1,..]`..`[if_button10,..]` for the variants the engine
//! still calls `InvButton1`..`InvButton10` -- the comment at
//! `rs-engine/rs-vm/src/trigger.rs:167` records that mapping and the byte values
//! it uses. `[if_button,..]` with no digit is the separate `IfButton` variant.
//!
//! `[command,..]` is not a trigger: it is the signature declaration form, and it
//! appears only in `engine.rs2`. That is asserted rather than assumed.
//!
//! **Not covered.** That the trigger word the compiler accepts maps to the same
//! number the engine uses -- `rs-runec` is an external crate and its table is
//! not readable from here. This checks the *name*, which is what the removed
//! header got wrong.
//!
//! ```text
//! cargo test -p rs-pack --test script_trigger_headers_647
//! ```

#![cfg(since_647)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The one file that declares `[command,..]` signatures.
const ENGINE_RS2: &str = "engine.rs2";

/// Headers content writes that the engine spells differently. Rev-647 renamed
/// the inventory-button triggers after the client packets that carry them; see
/// the module docs.
const ALIASES: [(&str, &str); 10] = [
    ("ifbutton1", "invbutton1"),
    ("ifbutton2", "invbutton2"),
    ("ifbutton3", "invbutton3"),
    ("ifbutton4", "invbutton4"),
    ("ifbutton5", "invbutton5"),
    ("ifbutton6", "invbutton6"),
    ("ifbutton7", "invbutton7"),
    ("ifbutton8", "invbutton8"),
    ("ifbutton9", "invbutton9"),
    ("ifbutton10", "invbutton10"),
];

/// Trigger words that must be found, so a parse that reads nothing fails here
/// instead of passing vacuously.
const PINNED_HEADERS: [&str; 6] = ["proc", "label", "login", "opnpc1", "if_button1", "oploc1"];

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rs-pack sits directly under the repo root")
        .to_path_buf()
}

/// Every server script under `content/647/scripts`, recursively.
fn script_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path
                .extension()
                .is_some_and(|ext| ext == "rs2" || ext == "gs2")
            {
                out.push(path);
            }
        }
    }

    let root = repo_root().join("content/647/scripts");
    let mut out = Vec::new();
    walk(&root, &mut out);
    out.sort();
    assert!(
        out.len() > 50,
        "only {} .rs2 files found under {}; the walk is broken",
        out.len(),
        root.display()
    );
    out
}

/// `(file, line number, trigger word)` for every block header in the tree.
///
/// Headers sit at column 0; nothing else in a `.rs2` starts with `[`.
fn headers() -> Vec<(String, usize, String)> {
    let root = repo_root();
    let mut out = Vec::new();
    for path in script_files() {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path.as_path())
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {rel}: {e}"));
        for (index, line) in text.lines().enumerate() {
            let Some(rest) = line.strip_prefix('[') else {
                continue;
            };
            let (trigger, _) = rest
                .split_once(',')
                .unwrap_or_else(|| panic!("{rel}:{}: header has no subject", index + 1));
            out.push((rel.clone(), index + 1, trigger.to_string()));
        }
    }
    out
}

/// The `ServerTriggerType` variant names, normalised to lowercase.
fn engine_triggers() -> BTreeSet<String> {
    let source = fs::read_to_string(repo_root().join("rs-engine/rs-vm/src/trigger.rs"))
        .expect("read rs-vm trigger.rs");
    let body = source
        .split_once("pub enum ServerTriggerType {")
        .expect("no ServerTriggerType enum")
        .1;
    let body = body.split_once("\n}").expect("unterminated enum").0;

    let mut out = BTreeSet::new();
    for line in body.lines() {
        let line = line.trim();
        let Some((name, value)) = line.split_once(" = ") else {
            continue;
        };
        // `Proc = 0,` -- anything else in the block is an attribute or comment.
        if !name.chars().all(|c| c.is_ascii_alphanumeric())
            || value.trim_end_matches(',').parse::<u8>().is_err()
        {
            continue;
        }
        out.insert(name.to_ascii_lowercase());
    }

    assert!(
        out.len() > 100,
        "only {} trigger variants parsed; the scan is broken, not the engine",
        out.len()
    );
    for expected in ["proc", "login", "opnpc1", "invbutton1", "ifbutton"] {
        assert!(
            out.contains(expected),
            "no `{expected}` in ServerTriggerType"
        );
    }
    out
}

/// `ai_queue5` -> `aiqueue5`, then the rev-647 `if_button*` renames.
fn normalise(trigger: &str) -> String {
    let flat = trigger.replace('_', "").to_ascii_lowercase();
    ALIASES
        .iter()
        .find(|(from, _)| *from == flat)
        .map(|(_, to)| (*to).to_string())
        .unwrap_or(flat)
}

/// `[command,..]` is a signature declaration, not a trigger, and it is confined
/// to `engine.rs2`.
///
/// The rest of this file skips those headers; if they ever spread to a content
/// script they would be skipped there too, unchecked.
#[test]
fn command_headers_live_only_in_engine_rs2() {
    let stray: Vec<String> = headers()
        .into_iter()
        .filter(|(file, _, trigger)| {
            trigger == "command" && !file.ends_with(&format!("/{ENGINE_RS2}"))
        })
        .map(|(file, line, _)| format!("{file}:{line}"))
        .collect();
    assert!(
        stray.is_empty(),
        "`[command,..]` declared outside {ENGINE_RS2}: {stray:?}"
    );
}

/// Every trigger header in `content/647` names a `ServerTriggerType` variant.
///
/// An unknown word is not a compile error: it produces a script with
/// `lookup = -1` that no engine path can reach, so the block is dead the moment
/// it is written and nothing says so.
#[test]
fn every_trigger_header_names_a_real_trigger() {
    let known = engine_triggers();
    let headers = headers();

    let seen: BTreeSet<&str> = headers
        .iter()
        .map(|(_, _, trigger)| trigger.as_str())
        .collect();
    for expected in PINNED_HEADERS {
        assert!(
            seen.contains(expected),
            "no `[{expected},..]` header anywhere in content/647 -- this test is not reading what it thinks"
        );
    }

    let unknown: Vec<String> = headers
        .iter()
        .filter(|(_, _, trigger)| trigger != "command" && !known.contains(&normalise(trigger)))
        .map(|(file, line, trigger)| format!("{file}:{line} [{trigger},..]"))
        .collect();

    assert!(
        unknown.is_empty(),
        "these headers name a trigger type the engine does not have, so they compile \
         to a lookup of -1 and can never fire: {unknown:?}"
    );
}

/// The mode switch is reached by name from the engine, not by a trigger.
///
/// Pinned because deleting `[switch_window_mode,_]` only holds if the engine
/// still runs the proc: the file has no trigger header left, so a handler that
/// stopped naming it would leave the whole script unreachable.
#[test]
fn the_mode_switch_is_reached_by_name() {
    let script = fs::read_to_string(
        repo_root().join("content/647/scripts/login_logout/scripts/switch_window_mode.rs2"),
    )
    .expect("read switch_window_mode.rs2");
    // Header position, not mere presence. A comment explaining why the header
    // was removed necessarily quotes it, and a bare `contains` cannot tell the
    // explanation from the thing it explains -- this assertion failed against
    // the very commit that removed the header, because that commit documented
    // what it had done. A trigger header only counts at the start of a line.
    assert!(
        !script
            .lines()
            .any(|line| line.trim_start().starts_with("[switch_window_mode,")),
        "the invented `[switch_window_mode,_]` trigger header is back"
    );
    assert!(
        script.contains("[proc,switch_window_mode]"),
        "switch_window_mode.rs2 no longer declares the proc"
    );

    // The other half of this -- that WindowStatus names the proc by its
    // bracketed header -- is guarded by `gameframe_window_mode_647`, which
    // arrives with the engine fix itself. Asserting it here too would mean two
    // tests pinning one fact, and this one would fail in a tree that has the
    // content change without the engine change.
}
