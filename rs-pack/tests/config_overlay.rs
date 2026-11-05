//! An authored config section lands on the cache's section of the same name
//! instead of colliding with it, so `_unpack/all.*` can be regenerated from
//! the cache and never edited by hand.
#![cfg(since_647)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rs_pack::pack::pack::{overlay_section, parse_config_sections};

fn kv(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn tree(name: &str, files: &[(&str, &str)]) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("config_overlay")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    let mut paths = Vec::new();
    for (rel, text) in files {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        paths.push(path);
    }
    paths.sort();
    paths
}

#[test]
fn a_patch_appends_server_only_keys_after_the_cache_lines() {
    let base = kv(&[("name", "Door"), ("op1", "Open"), ("model1", "1,2")]);
    let patch = kv(&[
        ("category", "door_closed"),
        ("param", "next_loc_stage,door_1"),
    ]);
    assert_eq!(
        overlay_section(base, patch),
        kv(&[
            ("name", "Door"),
            ("op1", "Open"),
            ("model1", "1,2"),
            ("category", "door_closed"),
            ("param", "next_loc_stage,door_1"),
        ])
    );
}

#[test]
fn a_patch_replaces_a_key_in_place_including_every_repeat() {
    let base = kv(&[
        ("name", "Man"),
        ("param", "a,1"),
        ("size", "1"),
        ("param", "b,2"),
    ]);
    let patch = kv(&[("param", "c,3")]);
    assert_eq!(
        overlay_section(base, patch),
        kv(&[("name", "Man"), ("param", "c,3"), ("size", "1")])
    );
}

#[test]
fn a_section_restating_every_key_replaces_in_its_own_order() {
    let base = kv(&[("name", "Tree"), ("op1", "Chop down")]);
    let full = kv(&[("op1", "Chop down"), ("name", "Tree"), ("category", "tree")]);
    assert_eq!(overlay_section(base, full.clone()), full);
}

#[test]
fn layers_merge_whichever_file_is_read_first() {
    // `aaa/` sorts before `_unpack/`, so the authored file is read first here;
    // the second case reverses that.
    for (name, authored) in [
        ("authored_first", "aaa/doors.loc"),
        ("authored_last", "zzz/doors.loc"),
    ] {
        let files = tree(
            name,
            &[
                (
                    "_unpack/all.loc",
                    "[door]\nname=Door\nop1=Open\n\n[wall]\nname=Wall\n",
                ),
                (authored, "[door]\ncategory=door_closed\n"),
            ],
        );
        let configs = parse_config_sections(&files, &HashMap::new());
        assert_eq!(
            configs["door"],
            kv(&[
                ("name", "Door"),
                ("op1", "Open"),
                ("category", "door_closed")
            ]),
            "{name}"
        );
        assert_eq!(configs["wall"], kv(&[("name", "Wall")]), "{name}");
    }
}

#[test]
fn an_authored_section_absent_from_the_cache_is_taken_whole() {
    let files = tree(
        "donor",
        &[
            ("_unpack/all.loc", "[door]\nname=Door\n"),
            ("donor/donor.loc", "[loc_864]\nname=Rock\nactive=yes\n"),
        ],
    );
    let configs = parse_config_sections(&files, &HashMap::new());
    assert_eq!(
        configs["loc_864"],
        kv(&[("name", "Rock"), ("active", "yes")])
    );
}

#[test]
#[should_panic(expected = "Two authored files cannot define one section")]
fn two_authored_files_for_one_section_is_still_an_error() {
    let files = tree(
        "two_authored",
        &[
            ("a/doors.loc", "[door]\ncategory=door_closed\n"),
            ("b/doors.loc", "[door]\ncategory=door_opened\n"),
        ],
    );
    parse_config_sections(&files, &HashMap::new());
}
