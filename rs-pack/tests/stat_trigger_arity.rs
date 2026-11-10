//! `[advancestat,..]` and `[changestat,..]` handlers must declare exactly the
//! parameters the engine passes.
//!
//! Both triggers are fired by the engine, not by a script. `change_stat` in
//! `rs-engine/src/script/player.rs` enqueues `[changestat]` with **no arguments**
//! (`None`) at every revision; `add_xp` does the same for `[advancestat]` before
//! 647, and from 647 passes one int, the number of levels the award crossed, for
//! 2011's "You've just advanced 2 Mining levels!" (`levelup.rs2`). A parameter
//! the engine does not pass is no error to `ScriptState::new`: it fills whatever
//! locals the script declares by `resize`-ing the local vectors to
//! `int_local_count` / `string_local_count`, so a declared-but-unsupplied
//! parameter reads back as `0` (or `""`). Nothing logs, nothing fails; the
//! handler simply computes with zeros.
//!
//! `content/647/scripts/player/xp.rs2` carried exactly that. It declared
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
//! 1. **Arity** (checked here): the handler took an argument the engine did not
//!    pass then (647's one int, the levels crossed, came later).
//! 2. **Semantics** (not checkable from text): `advancestat` is the *level-up*
//!    trigger, so even a zero-arg handler would only have run on level-up. Every
//!    earlier revision uses it that way -- `content/<rev>/scripts/levelup/
//!    levelup.rs2` hangs `@levelup(..)` off `[advancestat,<skill>]` for all 20
//!    skills -- and 647 is now consistent with them. The orb counter moved into
//!    `~givexp`, which counts the XP at grant time.
//!
//! **All revisions, one run.** The two triggers mean the same thing in every
//! revision the server supports, and the engine that fires them is shared, so
//! this walks every directory named in [`rs_ver::REVS`] rather than only the
//! `REV` the test binary was built for, holding each to its revision's
//! arguments ([`engine_params`]). `REV=225 cargo test` and `REV=647 cargo test`
//! therefore check the same seven content trees.
//!
//! **Not covered.** Whether a handler is *semantically* right for the trigger it
//! hangs off (point 2 above), and triggers other than these two. Other
//! engine-fired triggers pass arguments of their own; only these two are
//! asserted here, because each has a single call site whose arguments are
//! mirrored in [`engine_params`].
//!
//! ```text
//! cargo test -p rs-pack --test stat_trigger_arity
//! ```

use std::fs;
use std::path::{Path, PathBuf};

/// The triggers this test checks.
const STAT_TRIGGERS: [&str; 2] = ["advancestat", "changestat"];

/// The parameter types `rs-engine` passes `trigger` at `rev`, `,`-joined; `None`
/// when it passes no arguments. `add_xp` passes `[advancestat]` the levels crossed
/// from 647 (`#[cfg(since_647)]`); everything else is enqueued with `None`.
fn engine_params(trigger: &str, rev: &str) -> Option<&'static str> {
    let since_647 = rev.parse::<f64>().is_ok_and(|r| r >= 647.0);
    (trigger == "advancestat" && since_647).then_some("int")
}

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
    out.extend(runec::sources::script_files(dir, "rs2").unwrap());
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

/// The types of the parameter list a header line declares -- `](` with nothing
/// between -- `,`-joined; `None` when it declares none.
///
/// `[advancestat,attack] @levelup(..)` is a header followed by a *statement* on
/// the same line, which is why the check is for `(` immediately after `]` rather
/// than for a `(` anywhere on the line.
fn declared_params(line: &str) -> Option<String> {
    let rest = line[line.find(']')? + 1..].strip_prefix('(')?;
    let list = &rest[..rest.find(')')?];
    Some(
        list.split(',')
            .map(|p| p.split_whitespace().next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join(","),
    )
}

#[test]
fn stat_triggers_declare_the_engines_parameters() {
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
                if !STAT_TRIGGERS.contains(&trigger) {
                    continue;
                }
                checked += 1;
                if declared_params(line).as_deref() != engine_params(trigger, rev) {
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
        "found only {checked} `{STAT_TRIGGERS:?}` headers across {revisions_walked} \
         revisions; expected at least {MIN_HEADERS}. The walk is broken, not the content."
    );
    assert!(
        offenders.is_empty(),
        "these handlers declare other parameters than the engine passes, so the locals \
         read back as 0 with no error:\n  {}",
        offenders.join("\n  ")
    );
}
