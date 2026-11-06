//! OpenRS2 fetch: entry filtering, extraction, and XTEA key comparison.
//!
//! The offline tests build their own zips under `CARGO_TARGET_TMPDIR`. The
//! `#[ignore]`d ones talk to archive.openrs2.org and read `expected/`:
//! `REV=647 cargo test -p rs-pack --test openrs2 -- --ignored`
#![cfg(since_647)]

use rs_io::js5::Js5Store;
use rs_io::js5::compression::{CompressionType, compress};
use rs_io::js5::reference::{FileInfo, GroupInfo, ReferenceTable, encode_reference_table};
use rs_pack::archive::ARCHIVE_IDS;
use rs_pack::openrs2::{self, KeyDiff, XteaKey};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::from_utf8;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("openrs2")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.add_directory("cache/", options).unwrap();
    for (name, bytes) in entries {
        zip.start_file(*name, options).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn only_dat2_cache_files_are_accepted() {
    assert_eq!(
        openrs2::cache_file_name("cache/main_file_cache.dat2").as_deref(),
        Some("main_file_cache.dat2")
    );
    assert_eq!(
        openrs2::cache_file_name("cache/main_file_cache.idx255").as_deref(),
        Some("main_file_cache.idx255")
    );
    assert_eq!(
        openrs2::cache_file_name("main_file_cache.idx0").as_deref(),
        Some("main_file_cache.idx0")
    );
    for rejected in [
        "cache/main_file_cache.idx",
        "cache/main_file_cache.idx12a",
        "cache/main_file_cache.dat",
        "cache/readme.txt",
        "cache/",
        "",
    ] {
        assert_eq!(openrs2::cache_file_name(rejected), None, "{rejected:?}");
    }
}

#[test]
fn a_traversing_entry_lands_inside_the_output_by_its_bare_name() {
    assert_eq!(
        openrs2::cache_file_name("../../main_file_cache.idx3").as_deref(),
        Some("main_file_cache.idx3")
    );
    assert_eq!(
        openrs2::cache_file_name("..\\..\\main_file_cache.dat2").as_deref(),
        Some("main_file_cache.dat2")
    );
}

#[test]
fn disk_zip_is_extracted_flat_and_filtered() {
    let dir = scratch("extract");
    let zip = dir.join("disk.zip");
    write_zip(
        &zip,
        &[
            ("cache/main_file_cache.dat2", b"dat2 bytes"),
            ("cache/main_file_cache.idx255", b"master"),
            ("cache/main_file_cache.idx7", b"models"),
            ("cache/notes.txt", b"not a cache file"),
            ("../../main_file_cache.idx3", b"escapes"),
        ],
    );
    let out = dir.join("expected");

    let written = openrs2::extract_disk_zip(&zip, &out).unwrap();

    assert_eq!(
        written,
        [
            "main_file_cache.dat2",
            "main_file_cache.idx255",
            "main_file_cache.idx3",
            "main_file_cache.idx7"
        ]
    );
    assert_eq!(
        fs::read(out.join("main_file_cache.dat2")).unwrap(),
        b"dat2 bytes"
    );
    assert_eq!(
        fs::read(out.join("main_file_cache.idx3")).unwrap(),
        b"escapes"
    );
    assert!(!out.join("notes.txt").exists());
    assert!(!dir.join("main_file_cache.idx3").exists());
    let mut on_disk: Vec<_> = fs::read_dir(&out)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    on_disk.sort();
    assert_eq!(on_disk, written);
}

#[test]
fn a_zip_without_dat2_is_refused() {
    let dir = scratch("no_dat2");
    let zip = dir.join("disk.zip");
    write_zip(&zip, &[("cache/main_file_cache.dat", b"old engine")]);
    let err = openrs2::extract_disk_zip(&zip, &dir.join("expected")).unwrap_err();
    assert!(err.to_string().contains("no main_file_cache.dat2"), "{err}");
}

#[test]
fn keys_parse_from_the_api_shape_and_the_committed_shape() {
    let api = r#"[{"archive":5,"group":1,"name_hash":-1153472937,"name":"l40_55",
                   "mapsquare":10295,"key":[88121581,-1749097894,1831922082,-1885905985]}]"#;
    let committed = r#"[{"archive":5,"group":1,
                   "key":[88121581,-1749097894,1831922082,-1885905985],"name_hash":-1153472937}]"#;
    let expected = vec![XteaKey {
        archive: 5,
        group: 1,
        key: [88121581, -1749097894, 1831922082, -1885905985],
    }];
    assert_eq!(openrs2::parse_keys(api).unwrap(), expected);
    assert_eq!(openrs2::parse_keys(committed).unwrap(), expected);
}

#[test]
fn key_diff_reports_each_kind_of_disagreement() {
    let k = |group, first| XteaKey {
        archive: 5,
        group,
        key: [first, 0, 0, 0],
    };
    let local = [k(1, 10), k(2, 20), k(3, 30)];
    let remote = [k(1, 10), k(2, 99), k(4, 40)];
    assert_eq!(
        openrs2::diff_keys(&local, &remote),
        KeyDiff {
            only_local: vec![(5, 3)],
            only_remote: vec![(5, 4)],
            different: vec![(5, 2)],
        }
    );
    assert!(openrs2::diff_keys(&local, &local).is_empty());
}

/// Integration tests run with cwd = rs-pack/; the repo root is one up.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
#[ignore = "network: archive.openrs2.org, and needs expected/ from `cargo fetch-expected`"]
fn fetched_cache_matches_openrs2_reference_tables() {
    let id = openrs2::default_cache_id().unwrap();
    let mismatches = openrs2::verify_reference_tables(
        &repo_root().join("expected"),
        id,
        ARCHIVE_IDS,
        &Path::new(env!("CARGO_TARGET_TMPDIR")).join("openrs2-api"),
    )
    .unwrap();
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

/// Every key OpenRS2 has validated is committed unchanged, and decrypts its
/// group out of `expected/` -- gzip and bzip2 both reject a wrong key on
/// decompress, and the flipped-bit control below proves the check can fail.
///
/// The committed file knows 29 more keys than OpenRS2. They are for landscape
/// groups that exist in 786 but under keys nobody has; their squares were
/// rebuilt from donor caches, and `pack/other/map.rs` re-encrypts them with the
/// committed key. So they open *our* assembled cache, not 786 -- that is the
/// CRC gate's to check, not this test's.
#[test]
#[ignore = "network: archive.openrs2.org, and needs expected/ from `cargo fetch-expected`"]
fn committed_keys_hold_every_openrs2_key_and_each_one_decrypts() {
    let id = openrs2::default_cache_id().unwrap();
    let remote = openrs2::get_bytes(&openrs2::keys_url(id)).unwrap();
    let remote = openrs2::parse_keys(from_utf8(&remote).unwrap()).unwrap();
    let local =
        fs::read_to_string(repo_root().join(rs_pack::CONTENT_DIR).join("keys.json")).unwrap();
    let local = openrs2::parse_keys(&local).unwrap();

    let diff = openrs2::diff_keys(&local, &remote);
    assert!(
        diff.only_remote.is_empty(),
        "missing locally: {:?}",
        diff.only_remote
    );
    assert!(diff.different.is_empty(), "disagree: {:?}", diff.different);
    assert_eq!(diff.only_local.len(), 29, "{:?}", diff.only_local);

    let mut store = Js5Store::open(repo_root().join("expected")).unwrap();
    let mut bad = Vec::new();
    for k in &remote {
        let data = store.read_group_xtea(k.archive as u8, k.group, &k.key);
        if !matches!(data, Ok(Some(ref d)) if !d.is_empty()) {
            bad.push((k.archive, k.group, data.err().map(|e| e.to_string())));
        }
    }
    assert!(
        bad.is_empty(),
        "{} OpenRS2 keys do not decrypt: {bad:#?}",
        bad.len()
    );

    let k = &remote[0];
    let wrong = [k.key[0] ^ 1, k.key[1], k.key[2], k.key[3]];
    assert!(
        store
            .read_group_xtea(k.archive as u8, k.group, &wrong)
            .is_err(),
        "a wrong key decrypted ({}, {}) -- the check has no teeth",
        k.archive,
        k.group
    );
}

fn table(groups: &[(u32, u32, u32)]) -> ReferenceTable {
    ReferenceTable {
        archive_id: 7,
        format: 6,
        version: 1,
        named: false,
        whirlpooled: false,
        groups: groups
            .iter()
            .map(|&(id, crc, version)| GroupInfo {
                id,
                name_hash: -1,
                crc,
                whirlpool: None,
                version,
                size: 0,
                file_count: 1,
                files: vec![FileInfo {
                    id: 0,
                    name_hash: None,
                }],
            })
            .collect(),
    }
}

#[test]
fn table_diff_sorts_each_group_into_one_kind() {
    let ours = table(&[(1, 10, 1), (2, 20, 1), (3, 30, 1), (4, 40, 1)]);
    let theirs = table(&[(1, 10, 1), (2, 99, 1), (3, 30, 2), (5, 50, 1)]);
    let diff = openrs2::diff_reference_tables(&ours, &theirs);
    assert_eq!(
        diff,
        openrs2::TableDiff {
            only_ours: vec![4],
            only_theirs: vec![5],
            crc: vec![2],
            version: vec![3],
            header: false,
        }
    );
    assert_eq!(diff.groups(), [2, 3, 4, 5]);
    assert!(!diff.groups_agree());
}

#[test]
fn identical_groups_under_a_different_header_still_agree() {
    let ours = table(&[(1, 10, 1)]);
    let mut theirs = table(&[(1, 10, 1)]);
    theirs.version = 2;
    let diff = openrs2::diff_reference_tables(&ours, &theirs);
    assert!(diff.header);
    assert!(diff.groups_agree());
}

#[test]
fn a_reference_container_round_trips_through_the_parser() {
    let original = table(&[(0, 0xdead_beef, 3), (9, 1, 4)]);
    let encoded = encode_reference_table(&original);
    let container = compress(&encoded, CompressionType::Gzip).unwrap();
    let parsed = openrs2::parse_reference_container(&container).unwrap();
    assert!(openrs2::diff_reference_tables(&original, &parsed).groups_agree());
    assert!(!openrs2::diff_reference_tables(&original, &parsed).header);
}
