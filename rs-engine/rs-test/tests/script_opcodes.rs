//! Every command the content calls runs.
//!
//! A command declared in `engine.rs2` compiles whether or not the VM has a
//! handler for it at this revision; the gap only shows when a player reaches
//! the call (2011's dialogue options all failed on `if_setresumebuttons`,
//! which the VM handles before rev 254 only). This walks every compiled
//! script, tests included, and fails on any opcode the engine's registry has
//! no handler for.

#![cfg(since_647)]

use std::collections::BTreeMap;

use rs_test::runner;

/// Known gaps, each with its reason; anything else fails.
const KNOWN: &[(u16, &str)] = &[
    // 2011's resting (the musician's Listen-to): post-289, with the VM's
    // other 2011-only commands (plan 2.6).
    (2183, "p_rest"),
];

#[test]
fn every_opcode_the_scripts_use_has_a_handler() {
    runner::on_loaded_engine(|engine| {
        let mut missing: BTreeMap<u16, Vec<String>> = BTreeMap::new();
        for script in engine.scripts.scripts.iter().flatten() {
            for &op in script.opcodes.iter() {
                if !engine.ops.contains(op) && !KNOWN.iter().any(|(k, _)| *k == op) {
                    let users = missing.entry(op).or_default();
                    if !users.iter().any(|u| *u == *script.info.name) {
                        users.push(script.info.name.to_string());
                    }
                }
            }
        }
        let report: Vec<String> = missing
            .iter()
            .map(|(op, users)| format!("{op}: {} script(s), e.g. {}", users.len(), users[0]))
            .collect();
        assert!(
            missing.is_empty(),
            "opcodes with no handler:\n  {}",
            report.join("\n  ")
        );
        for (op, name) in KNOWN {
            assert!(
                !engine.ops.contains(*op),
                "{name} ({op}) has a handler now: take it off KNOWN"
            );
        }
    });
}
