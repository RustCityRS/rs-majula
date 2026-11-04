//! Every revision's RuneScript compiles and lints.
//!
//! `pack_all` compiles content at server startup and, crucially, treats failure
//! as a **warning**: `startup.rs` matches on `runec::compile(..)` and calls
//! `warn!` on `Err`, then carries on and builds the cache anyway. A revision
//! whose content stopped compiling therefore boots a server with a stale (or
//! missing) `script.dat` rather than refusing to start, and the first symptom is
//! a script that silently does not run. The lint step is warned-past in the same
//! way. This test is the gate that startup deliberately is not.
//!
//! **All revisions, one run.** The compiler is shared across revisions and the
//! content trees are all on disk, so this walks every directory named in
//! [`rs_ver::REVS`] rather than only the `REV` the binary was built for -- one
//! `cargo test` covers all seven. `compile_memory` returns the
//! `(script.dat, script.idx)` bytes instead of writing them, so nothing on disk
//! is touched and the run does not race the server's own `data/` output.
//!
//! Roughly 3s per revision, ~20s in total.
//!
//! **Not covered.** That a compiled script does what it should, or that its
//! trigger header names a trigger the engine can fire -- a header the compiler
//! does not recognise still compiles, to a script with `lookup = -1` that can
//! never run. `script_trigger_headers_647.rs` covers that; `stat_trigger_arity.rs`
//! covers handlers whose declared parameters the engine never passes.
//!
//! ```text
//! cargo test -p rs-pack --test content_compiles
//! ```

use std::path::{Path, PathBuf};

/// Smallest plausible `script.dat`. The leanest tree (647, ~80 files) produces
/// just over 200KB; this only has to be large enough that a compile which
/// silently found no scripts fails here instead of passing.
const MIN_DAT_BYTES: usize = 100_000;

/// Smallest plausible `script.idx`, on the same reasoning (647: ~4.7KB).
const MIN_IDX_BYTES: usize = 1_000;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rs-pack sits directly under the repo root")
        .to_path_buf()
}

#[test]
fn every_revision_compiles_and_lints() {
    let root = repo_root();
    let mut failures = Vec::new();

    for rev in rs_ver::REVS {
        let content = root.join("content").join(rev);
        let scripts = content.join("scripts");
        let pack = content.join("pack");
        assert!(
            scripts.is_dir(),
            "revision {rev} is listed in rs_ver::REVS but {} does not exist",
            scripts.display()
        );

        match runec::compile_memory(&scripts, Some(&pack), true) {
            Ok((dat, idx)) => {
                if dat.len() < MIN_DAT_BYTES || idx.len() < MIN_IDX_BYTES {
                    failures.push(format!(
                        "{rev}: compiled, but suspiciously small -- script.dat {} bytes, \
                         script.idx {} bytes. The scripts directory is probably not being read.",
                        dat.len(),
                        idx.len()
                    ));
                }
            }
            Err(e) => failures.push(format!("{rev}: {e}")),
        }
    }

    assert!(
        failures.is_empty(),
        "content failed to compile or lint:\n  {}",
        failures.join("\n  ")
    );
}
