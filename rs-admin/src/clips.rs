use rs_admin_api::ReplaySummary;
use std::path::{Path, PathBuf};

/// Recordings the console keeps because a case has a clip of them: the
/// worlds prune recordings after a few days, and evidence has to outlive
/// that. Each is its replay file and its summary, by the recording's id.
/// Never deleted, as the case log keeps every clip ever added.
#[derive(Clone)]
pub struct Clips {
    dir: PathBuf,
}

/// How long a replay file says it is (its header's `durationMs`), if it's
/// readable.
pub fn file_duration(file: &[u8]) -> Option<u64> {
    let len = u32::from_be_bytes(file.get(5..9)?.try_into().ok()?) as usize;
    let header: serde_json::Value = serde_json::from_slice(file.get(9..9 + len)?).ok()?;
    header.get("durationMs")?.as_u64()
}

/// Writes beside `path` and renames over it, so a crash never leaves it half
/// written.
fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

impl Clips {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Ids are digits only (see `checked_id`), so they're safe as names.
    fn path(&self, id: &str, extension: &str) -> PathBuf {
        self.dir.join(format!("{id}.{extension}"))
    }

    /// Keeps `file` as recording `id`'s, unless a longer copy is kept
    /// already (one taken later in a session still going).
    pub fn keep(&self, summary: &ReplaySummary, file: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let path = self.path(&summary.id, "replay");
        let longer = std::fs::read(&path)
            .ok()
            .is_some_and(|old| file_duration(&old).unwrap_or(0) > file_duration(file).unwrap_or(0));
        if longer {
            return Ok(());
        }
        write(&path, file)?;
        write(
            &self.path(&summary.id, "json"),
            &serde_json::to_vec_pretty(summary).map_err(std::io::Error::other)?,
        )
    }

    pub fn file(&self, id: &str) -> Option<Vec<u8>> {
        std::fs::read(self.path(id, "replay")).ok()
    }

    pub fn summary(&self, id: &str) -> Option<ReplaySummary> {
        let text = std::fs::read(self.path(id, "json")).ok()?;
        serde_json::from_slice(&text).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(duration_ms: u64) -> Vec<u8> {
        let header = format!(r#"{{"durationMs":{duration_ms},"notes":[]}}"#);
        let mut out = b"RSRPL".to_vec();
        out.extend((header.len() as u32).to_be_bytes());
        out.extend(header.as_bytes());
        out.extend([1, 2, 3]);
        out
    }

    fn summary(id: &str) -> ReplaySummary {
        ReplaySummary {
            id: id.to_owned(),
            player: "Bob".into(),
            node: 10,
            revision: "289".into(),
            started_ms: 1,
            ended_ms: None,
            updated_ms: 1,
            duration_ms: 1,
            bytes: 1,
            complete: true,
            score: 0,
            flags: Vec::new(),
            stats: String::new(),
        }
    }

    #[test]
    fn keeps_the_longest_copy() {
        let dir = std::env::temp_dir().join(format!("rs-admin-clips-{}", std::process::id()));
        let clips = Clips::new(dir.clone());
        assert_eq!(file_duration(&file(5000)), Some(5000));
        assert_eq!(file_duration(b"short"), None);
        clips.keep(&summary("12"), &file(5000)).unwrap();
        clips.keep(&summary("12"), &file(3000)).unwrap();
        assert_eq!(file_duration(&clips.file("12").unwrap()), Some(5000));
        clips.keep(&summary("12"), &file(9000)).unwrap();
        assert_eq!(file_duration(&clips.file("12").unwrap()), Some(9000));
        assert_eq!(clips.summary("12").unwrap().player, "Bob");
        assert!(clips.file("13").is_none());
        let _ = std::fs::remove_dir_all(dir);
    }
}
