#[cfg(any(since_647, test))]
use anyhow::bail;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, io, process};
use tracing::warn;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// A copy of the pack dir for one compile alone, so worlds packing at once
/// never compile against each other's half-written files.
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

    #[cfg(any(since_647, test))]
    pub fn check_unchanged(&self, pack: &Path, name: &str) -> Result<()> {
        let original = fs::read_to_string(pack.join(name))?;
        let compiled = fs::read_to_string(self.dir.join(name))?;
        if original != compiled {
            let added: Vec<_> = compiled
                .lines()
                .filter(|line| !original.lines().any(|old| old == *line))
                .collect();
            bail!(
                "{name} changed during compilation with --verify true:\n{}",
                added.join("\n")
            );
        }
        Ok(())
    }

    #[cfg(any(since_647, test))]
    pub fn pin(&self, pack: &Path, name: &str, verify: bool) -> Result<()> {
        if verify {
            return self.check_unchanged(pack, name);
        }
        let compiled = fs::read(self.dir.join(name))?;
        let path = pack.join(name);
        if !fs::read(&path).is_ok_and(|original| original == compiled) {
            replace(&path, &compiled).with_context(|| format!("updating {}", path.display()))?;
        }
        Ok(())
    }
}

impl Drop for PrivatePack {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_dir_all(&self.dir) {
            warn!("Couldn't remove {}: {e}", self.dir.display());
        }
    }
}

/// Replaces a registry atomically so concurrent readers never see a partial file.
pub fn replace(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(format!(
        ".{}-{}.tmp",
        process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let tmp = PathBuf::from(tmp);
    let result = fs::write(&tmp, contents).and_then(|()| fs::rename(&tmp, path));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Copies registry files without unfinished temporary files.
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

#[cfg(test)]
mod tests {
    use super::*;
    use runec::clientscript;

    #[test]
    fn clientscript_growth_requires_verify_false() {
        let dir = env::temp_dir().join(format!("rs-majula-clientscript-ids-{}", process::id()));
        let scripts = dir.join("scripts");
        let pack = dir.join("pack");
        fs::create_dir_all(&scripts).unwrap();
        fs::create_dir_all(&pack).unwrap();
        fs::write(pack.join("clientscript.pack"), "0=[proc,existing]\n").unwrap();
        fs::write(
            pack.join("cs2_command.pack"),
            "0=PushConstantInt\n21=Return\n",
        )
        .unwrap();
        fs::write(scripts.join("engine.cs2"), "").unwrap();
        fs::create_dir_all(scripts.join("bank/client")).unwrap();
        fs::write(
            scripts.join("bank/client/new.cs2"),
            "[proc,new_script]()(int) return(1);\n",
        )
        .unwrap();
        let private = PrivatePack::copy(&pack).unwrap();
        private.check_unchanged(&pack, "clientscript.pack").unwrap();
        clientscript::compile_directory(&scripts, private.path()).unwrap();
        let error = private
            .pin(&pack, "clientscript.pack", true)
            .unwrap_err()
            .to_string();
        assert!(error.contains("1=[proc,new_script]"), "{error}");
        assert_eq!(
            fs::read_to_string(pack.join("clientscript.pack")).unwrap(),
            "0=[proc,existing]\n"
        );
        private.pin(&pack, "clientscript.pack", false).unwrap();
        private.check_unchanged(&pack, "clientscript.pack").unwrap();
        fs::remove_file(pack.join("clientscript.pack")).unwrap();
        assert!(private.pin(&pack, "clientscript.pack", true).is_err());
        assert!(!pack.join("clientscript.pack").exists());
        private.pin(&pack, "clientscript.pack", false).unwrap();
        private.check_unchanged(&pack, "clientscript.pack").unwrap();
        fs::remove_dir_all(&dir).unwrap();
    }
}
