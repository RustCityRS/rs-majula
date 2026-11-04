//! `[advancestat,..]` and `[changestat,..]` handlers must declare no parameters.
//!
//! Both triggers are fired by the engine, not by a script, and both are enqueued
//! with **no arguments** -- `add_xp` and `change_stat` in
//! `rs-engine/src/engine.rs` pass `None` for the `args` field of the queued
//! entry. `ScriptState::new` does not treat that as an error: it fills whatever
//! locals the script declares by `resize`-ing the local vectors to
//! `int_local_count` / `string_local_count`, so a declared-but-unsupplied
//! parameter reads back as `0` (or `""`). Nothing logs, nothing fails; the
//! handler simply computes with zeros.
//!
//! `content/647/scripts/player/scripts/xp.rs2` carried exactly that. It declared
//!
//! ```text
//! [advancestat,_](int $xp)
//! %xp_counter = calc(%xp_counter + $xp * 10);
//! ```
//!
//! and the XP-counter orb never moved -- `$xp` was always the zero-fill, so every
//! firing added `0 * 10`. Two separate mistakes compounded, and this test catches
//! the one that is mechanically checkable:
//!
//! 1. **Arity** (checked here): the handler took an argument the engine does not
//!    pass.
//! 2. **Semantics** (not checkable from text): `advancestat` is the *level-up*
//!    trigger, so even a zero-arg handler would only have run on level-up. Every
//!    earlier revision uses it that way -- `content/<rev>/scripts/levelup/scripts/
//!    levelup.rs2` hangs `@levelup(..)` off `[advancestat,<skill>]` for all 20
//!    skills -- and 647 is now consistent with them. The orb counter moved into
//!    `~givexp`, which counts the XP at grant time.
//!
//! **All revisions, one run.** The two triggers mean the same thing in every
//! revision the server supports, and the engine that fires them is shared, so
//! this walks every directory named in [`rs_ver::REVS`] rather than only the
//! `REV` the test binary was built for. `REV=225 cargo test` and
//! `REV=647 cargo test` therefore check the same seven content trees.
//!
//! **Not covered.** Whether a zero-arg handler is *semantically* right for the
//! trigger it hangs off (point 2 above), and triggers other than these two.
//! Other engine-fired triggers do pass arguments; only these two are asserted
//! here, because only these two are enqueued with `None` at every call site.
//!
//! ```text
//! cargo test -p rs-pack --test stat_trigger_arity
//! ```

use std::fs;
use std::path::{Path, PathBuf};

/// The triggers `rs-engine` enqueues with `None` for arguments.
const ARGLESS_TRIGGERS: [&str; 2] = ["advancestat", "changestat"];

/// Below this, the walk found too little to be believed and the test is failing
/// vacuously rather than passing. Six revisions carry 20 `[advancestat,<skill>]`
/// level-up handlers each; 647 carries one `[changestat,hitpoints]`.
const MIN_HEADERS: usize = 100;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rs-pack sits directly under the repo root")
        .to_path_buf()
}

/// Every `.rs2` under `dir`, recursively.
fn script_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            script_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs2") {
            out.push(path);
        }
    }
}

/// The trigger word of a header line, if the line opens one.
///
/// A header is `[<trigger>,<subject>]`, always at column 0 -- an indented `[` is
/// inside a block. The subject may itself contain `:` or `_`, so the trigger is
/// everything between `[` and the first `,`.
fn header_trigger(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('[')?;
    let comma = rest.find(',')?;
    let bracket = rest.find(']')?;
    (comma < bracket).then(|| &rest[..comma])
}

/// Whether a header line declares a parameter list -- `](` with nothing between.
///
/// `[advancestat,attack] @levelup(..)` is a header followed by a *statement* on
/// the same line, which is why the check is for `(` immediately after `]` rather
/// than for a `(` anywhere on the line.
fn declares_parameters(line: &str) -> bool {
    line.find(']')
        .and_then(|i| line[i + 1..].chars().next())
        .is_some_and(|c| c == '(')
}

#[test]
fn argless_stat_triggers_declare_no_parameters() {
    let root = repo_root();
    let mut offenders = Vec::new();
    let mut checked = 0usize;
    let mut revisions_walked = 0usize;

    for rev in rs_ver::REVS {
        let scripts = root.join("content").join(rev).join("scripts");
        assert!(
            scripts.is_dir(),
            "revision {rev} is listed in rs_ver::REVS but {} does not exist",
            scripts.display()
        );
        revisions_walked += 1;

        let mut files = Vec::new();
        script_files(&scripts, &mut files);
        for file in files {
            let text = fs::read_to_string(&file)
                .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
            for (n, line) in text.lines().enumerate() {
                let Some(trigger) = header_trigger(line) else {
                    continue;
                };
                if !ARGLESS_TRIGGERS.contains(&trigger) {
                    continue;
                }
                checked += 1;
                if declares_parameters(line) {
                    let shown = file.strip_prefix(&root).unwrap_or(&file);
                    offenders.push(format!(
                        "{}:{}: {}",
                        shown.display(),
                        n + 1,
                        line.trim_end()
                    ));
                }
            }
        }
    }

    assert_eq!(
        revisions_walked,
        rs_ver::REVS.len(),
        "every revision in rs_ver::REVS must be walked"
    );
    assert!(
        checked >= MIN_HEADERS,
        "found only {checked} `{ARGLESS_TRIGGERS:?}` headers across {revisions_walked} \
         revisions; expected at least {MIN_HEADERS}. The walk is broken, not the content."
    );
    assert!(
        offenders.is_empty(),
        "these handlers declare parameters the engine never passes, so the locals \
         read back as 0 with no error:\n  {}",
        offenders.join("\n  ")
    );
}
