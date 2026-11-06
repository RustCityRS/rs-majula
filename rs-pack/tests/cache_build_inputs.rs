//! The JS5 build is skipped wholesale while its stamp is newer than every
//! source it reads, so each source has to be among `cache_build_inputs`.
//! Map squares were not: the list watched `maps/_unpack/*.map`, a format
//! nothing reads, so an edit to a landscape `.jm2` never rebuilt the cache.
//!
//! A source that *disappears* has to count too. Removing or renaming a file
//! leaves every remaining one as old as before, so only the directory's own
//! mtime can say anything changed.
#![cfg(since_647)]

use rs_pack::startup::{cache_build_inputs, write_stamp};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::thread::sleep;
use std::time::{Duration, SystemTime};

fn content(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("cache_build_inputs")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::create_dir_all(dir.join("maps").join("_unpack").join("encrypted")).unwrap();
    dir
}

fn write_at(path: &Path, when: SystemTime) {
    fs::write(path, b"x").unwrap();
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(when)
        .unwrap();
}

/// A file time later than any directory these tests create, so the file's
/// own mtime is what decides.
fn at(secs: u64) -> SystemTime {
    static BASE: OnceLock<SystemTime> = OnceLock::new();
    *BASE.get_or_init(|| SystemTime::now() + Duration::from_secs(3600)) + Duration::from_secs(secs)
}

/// A file time earlier than any directory these tests create.
fn long_ago() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000)
}

/// Comfortably past the filesystem's timestamp granularity.
fn tick() {
    sleep(Duration::from_millis(50));
}

fn squares(dir: &Path) -> PathBuf {
    dir.join("maps").join("_unpack")
}

#[test]
fn a_deleted_square_is_a_change() {
    let dir = content("deleted");
    write_at(&dir.join("scripts").join("all.npc"), long_ago());
    write_at(&squares(&dir).join("l50_50.jm2"), long_ago());
    write_at(&squares(&dir).join("l50_51.jm2"), long_ago());
    let before = cache_build_inputs(&dir).unwrap();
    tick();
    fs::remove_file(squares(&dir).join("l50_51.jm2")).unwrap();
    assert!(cache_build_inputs(&dir).unwrap() > before);
}

#[test]
fn a_renamed_config_is_a_change() {
    let dir = content("renamed");
    write_at(&dir.join("scripts").join("wearpos.obj"), long_ago());
    let before = cache_build_inputs(&dir).unwrap();
    tick();
    fs::rename(
        dir.join("scripts").join("wearpos.obj"),
        dir.join("scripts").join("wearpos.obj.off"),
    )
    .unwrap();
    assert!(cache_build_inputs(&dir).unwrap() > before);
}

#[test]
fn an_edited_landscape_is_a_build_input() {
    let dir = content("landscape");
    write_at(&dir.join("scripts").join("all.npc"), at(0));
    write_at(&dir.join("maps").join("_unpack").join("l50_50.jm2"), at(10));
    assert_eq!(cache_build_inputs(&dir), Some(at(10)));
}

#[test]
fn an_opaque_encrypted_square_is_a_build_input() {
    let dir = content("encrypted");
    write_at(&dir.join("scripts").join("all.npc"), at(0));
    write_at(
        &dir.join("maps")
            .join("_unpack")
            .join("encrypted")
            .join("617.bin"),
        at(20),
    );
    assert_eq!(cache_build_inputs(&dir), Some(at(20)));
}

#[test]
fn an_unread_extension_is_not() {
    let dir = content("unread");
    write_at(&dir.join("scripts").join("all.npc"), at(0));
    write_at(&dir.join("maps").join("_unpack").join("notes.txt"), at(30));
    assert_eq!(cache_build_inputs(&dir), Some(at(0)));
}

#[test]
fn every_config_source_is_a_build_input() {
    let dir = content("configs");
    for (i, ext) in [
        "seq", "bas", "light", "config48", "constant", "mesanim", "varn", "vars",
    ]
    .into_iter()
    .enumerate()
    {
        let when = at(i as u64);
        write_at(&dir.join("scripts").join(format!("all.{ext}")), when);
        assert_eq!(cache_build_inputs(&dir), Some(when), "{ext}");
    }
}

/// A stamp is only as good as its mtime: rewriting one must move it to now.
/// `.anim_stamp` was written as an empty file, and on Windows truncating an
/// empty file to empty leaves the mtime where it was -- so it stayed at its
/// first write and every start after a newer input repacked every frame.
#[test]
fn rewriting_a_stamp_moves_it_to_now() {
    let dir = content("stamp");
    let stamp = dir.join(".anim_stamp");
    fs::write(&stamp, []).unwrap();
    File::options()
        .write(true)
        .open(&stamp)
        .unwrap()
        .set_modified(long_ago())
        .unwrap();
    let before = SystemTime::now() - Duration::from_secs(1);
    write_stamp(&stamp).unwrap();
    let after = fs::metadata(&stamp).unwrap().modified().unwrap();
    assert!(after >= before, "the stamp kept its old mtime: {after:?}");
}
