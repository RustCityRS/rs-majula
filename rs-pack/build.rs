use std::fs;
use std::hash::{DefaultHasher, Hasher};
use std::path::{Path, PathBuf};

fn main() {
    rs_ver::compile_target_revision();
    let mut hasher = DefaultHasher::new();
    let mut watched = vec![
        PathBuf::from("src"),
        PathBuf::from("../Cargo.lock"),
        PathBuf::from("../Cargo.toml"),
    ];
    hash_tree(&mut hasher, Path::new("src"));
    hash_file(&mut hasher, Path::new("../Cargo.lock"));
    for dir in patched_dependencies() {
        let (src, manifest) = (dir.join("src"), dir.join("Cargo.toml"));
        hash_tree(&mut hasher, &src);
        hash_file(&mut hasher, &manifest);
        watched.extend([src, manifest]);
    }
    println!(
        "cargo::rustc-env=RS_PACK_FINGERPRINT={:016x}",
        hasher.finish()
    );
    for path in watched {
        println!("cargo::rerun-if-changed={}", path.display());
    }
}

/// Every file under `dir`: an edit to the packer, or to a dependency built
/// from a local checkout, starts the pack cache over.
fn hash_tree(hasher: &mut DefaultHasher, dir: &Path) {
    let mut files = Vec::new();
    collect(dir, &mut files);
    files.sort();
    for file in files {
        hash_file(hasher, &file);
    }
}

fn hash_file(hasher: &mut DefaultHasher, path: &Path) {
    hasher.write(path.to_string_lossy().as_bytes());
    hasher.write(&fs::read(path).unwrap_or_default());
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// The local checkouts `[patch.crates-io]` builds dependencies from, which
/// Cargo.lock doesn't version.
fn patched_dependencies() -> Vec<PathBuf> {
    let manifest = fs::read_to_string("../Cargo.toml").unwrap_or_default();
    let mut in_patch = false;
    let mut dirs = Vec::new();
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_patch = line == "[patch.crates-io]";
            continue;
        }
        if in_patch
            && let Some((_, rest)) = line.split_once("path")
            && let Some(start) = rest.find('"')
            && let Some(len) = rest[start + 1..].find('"')
        {
            dirs.push(Path::new("..").join(&rest[start + 1..start + 1 + len]));
        }
    }
    dirs
}
