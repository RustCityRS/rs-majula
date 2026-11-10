use rs_pack::pack::pack::{FileCache, parse_config_sections, parse_config_sections_cached};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn tree(name: &str, files: &[(&str, &str)]) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("config_sections")
        .join(name);
    files
        .iter()
        .map(|(name, text)| {
            let path = root.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, text).unwrap();
            path
        })
        .collect()
}

#[test]
fn repeated_headers_in_one_file_report_both_lines() {
    let files = tree(
        "same_file",
        &[(
            "quest.varp",
            "[arenaquest]\ntransmit=yes\n\n[arenaquest]\nscope=perm\n",
        )],
    );
    let error = parse_config_sections(&files, &HashMap::new())
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("quest.varp:4: Duplicate config [arenaquest]"),
        "{error}"
    );
    assert!(error.contains("quest.varp:1"), "{error}");
}

#[test]
fn repeated_headers_across_files_are_errors_in_both_loaders() {
    for first in ["quests/first.varp", "configs/_unpack/all.varp"] {
        let mut files = tree(
            first,
            &[
                (first, "[arenaquest]\ntransmit=yes\n"),
                ("quests/arena/quest.varp", "\n[arenaquest]\nscope=perm\n"),
            ],
        );
        for _ in 0..2 {
            let error = parse_config_sections(&files, &HashMap::new())
                .unwrap_err()
                .to_string();
            assert!(error.contains("Duplicate config [arenaquest]"), "{error}");
            for path in &files {
                assert!(error.contains(&path.display().to_string()), "{error}");
            }
            files.reverse();
        }
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("config_sections")
            .join(first);
        let cache = FileCache::new(&root);
        assert!(parse_config_sections_cached(&cache, "varp", &HashMap::new()).is_err());
    }
}

#[test]
fn one_section_preserves_order_repeated_properties_and_constants() {
    let files = tree(
        "properties",
        &[("test.struct", "[test]\nparam=a,^value\nparam=b,2\n")],
    );
    let constants = HashMap::from([("value".into(), "1".into())]);
    let sections = parse_config_sections(&files, &constants).unwrap();
    assert_eq!(
        sections["test"],
        vec![
            ("param".into(), "a,1".into()),
            ("param".into(), "b,2".into())
        ]
    );
}

#[test]
fn every_revision_defines_each_config_once() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../content");
    for rev in rs_ver::REVS {
        let cache = FileCache::new(&root.join(rev).join("scripts"));
        let mut count = 0;
        for kind in [
            "loc", "npc", "obj", "seq", "varp", "varpbit", "flo", "flu", "idk", "spotanim",
            "param", "dbrow", "dbtable", "enum", "struct", "hunt", "varn", "vars", "inv",
            "mesanim",
        ] {
            count += parse_config_sections(cache.collect(kind), &HashMap::new())
                .unwrap_or_else(|error| panic!("REV={rev}: {error}"))
                .len();
        }
        assert!(count > 100, "REV={rev}: no config sources found");
    }
}

#[cfg(since_647)]
#[test]
fn unpack_rejects_existing_configs_before_writing() {
    let files = tree(
        "unpack",
        &[("scripts/quest.varp", "[arenaquest]\nscope=perm\n")],
    );
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("config_sections/unpack");
    let before = fs::read(&files[0]).unwrap();
    let error = rs_pack::unpack::unpack_all(&root.join("missing-cache"), &root, &root.join("pack"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("already contains configs"), "{error}");
    assert_eq!(fs::read(&files[0]).unwrap(), before);
    assert!(!root.join("scripts/_unpack").exists());
    assert!(!root.join("pack").exists());
}
