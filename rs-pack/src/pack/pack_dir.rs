use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, io, process};
use tracing::warn;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A copy of the pack dir for one compile alone, so worlds packing at once
/// never compile against each other's half-written files. The ids a compile
/// gives go back to the pack dir with [`pin`](Self::pin).
pub struct PrivatePack {
    dir: PathBuf,
}

impl PrivatePack {
    pub fn copy(pack: &Path) -> Result<Self> {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        let dir = env::temp_dir().join(format!(
            "rs-pack-{}-{nanos}-{}",
            process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&dir).with_context(|| format!("creating {}", dir.display()))?;
        let private = Self { dir };
        copy_files(pack, &private.dir)
            .with_context(|| format!("copying {} to compile against", pack.display()))?;
        Ok(private)
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }

    /// Swaps file `name` into `pack` as the compile left it, if it changed.
    pub fn pin(&self, pack: &Path, name: &str) {
        let Ok(pinned) = fs::read(self.dir.join(name)) else {
            return;
        };
        let shipped = pack.join(name);
        if fs::read(&shipped).is_ok_and(|old| old == pinned) {
            return;
        }
        if let Err(e) = replace(&shipped, &pinned) {
            warn!("Couldn't update {}: {e}", shipped.display());
        }
    }
}

impl Drop for PrivatePack {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_dir_all(&self.dir) {
            warn!("Couldn't remove {}: {e}", self.dir.display());
        }
    }
}

/// Writes `path` by renaming a finished file over it, so readers in every
/// world see the old contents or the new, never part of them.
pub fn replace(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(
        ".{}-{}.tmp",
        process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let tmp = PathBuf::from(tmp);
    let written = fs::write(&tmp, contents).and_then(|()| fs::rename(&tmp, path));
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

/// Copies the files of `pack` into `to`, less any another world's
/// [`replace`] is still writing.
fn copy_files(pack: &Path, to: &Path) -> io::Result<()> {
    for entry in fs::read_dir(pack)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().ends_with(".tmp") || !entry.file_type()?.is_file() {
            continue;
        }
        fs::copy(entry.path(), to.join(name))?;
    }
    Ok(())
}
