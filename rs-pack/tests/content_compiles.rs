//! Compile and lint every revision listed in `rs_ver::REVS`.
//! The compiler returns bytecode in memory, but its registry generation can
//! still update `script.pack`. Trigger headers and arity have separate tests.

use std::path::{Path, PathBuf};

/// Reject a compile that silently found no scripts.
const MIN_DAT_BYTES: usize = 100_000;

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
