//! Every `#testscript` label in the revision's content passes.
//!
//! One test, because the labels share one engine: loading the map and
//! compiling the scripts dominates the run, and each label resets the world
//! before it starts. The report lists each label, so a failure names itself.

#![cfg(since_647)]

use rs_test::runner;

#[test]
fn every_testscript_label_passes() {
    let passed = runner::on_loaded_engine(|engine| {
        let labels = runner::discover_labels(&runner::content_dir().join("scripts"), None);
        assert!(!labels.is_empty(), "found no #testscript labels");
        let results: Vec<_> = labels
            .iter()
            .map(|l| runner::run_label(engine, l))
            .collect();
        runner::report(&results)
    });
    assert!(
        passed,
        "some #testscript labels failed; see the report above"
    );
}
