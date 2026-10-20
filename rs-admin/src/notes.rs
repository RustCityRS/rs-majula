use crate::users::{name_key, now_ms};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::warn;

/// The longest note.
pub const MAX_TEXT: usize = 1000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Note {
    pub id: u64,
    pub player: String,
    pub text: String,
    #[serde(default)]
    pub watch: bool,
    pub by: String,
    pub at_ms: u64,
}

/// One line of the log.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Logged {
    Added {
        #[serde(flatten)]
        note: Note,
    },
    Removed {
        id: u64,
        by: String,
        at_ms: u64,
    },
}

#[derive(Default)]
struct Inner {
    /// The notes that stand, oldest first.
    notes: Vec<Note>,
    next_id: u64,
}

impl Inner {
    fn apply(&mut self, logged: Logged) {
        match logged {
            Logged::Added { note } => {
                self.next_id = self.next_id.max(note.id + 1);
                self.notes.push(note);
            }
            Logged::Removed { id, .. } => self.notes.retain(|n| n.id != id),
        }
    }
}

pub struct Notes {
    path: PathBuf,
    inner: Mutex<Inner>,
}

impl Notes {
    /// Opens the log at `path` and rebuilds the notes from it.
    pub fn open(path: PathBuf) -> Self {
        let mut inner = Inner {
            next_id: 1,
            ..Inner::default()
        };
        if let Ok(text) = std::fs::read_to_string(&path) {
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                match serde_json::from_str::<Logged>(line) {
                    Ok(logged) => inner.apply(logged),
                    Err(e) => warn!("Skipping an unreadable note log entry: {}", e),
                }
            }
        }
        Self {
            path,
            inner: Mutex::new(inner),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Writes `logged` and syncs it before it takes.
    fn log(&self, inner: &mut Inner, logged: Logged) -> Result<(), String> {
        let write = || -> std::io::Result<()> {
            if let Some(dir) = self.path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let mut line = serde_json::to_vec(&logged)?;
            line.push(b'\n');
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?;
            file.write_all(&line)?;
            file.sync_data()
        };
        write().map_err(|e| {
            warn!(
                "Failed to write the note log {}: {}",
                self.path.display(),
                e
            );
            "The console couldn't save that. Try again.".to_owned()
        })?;
        inner.apply(logged);
        Ok(())
    }

    pub fn add(&self, by: &str, player: &str, text: &str, watch: bool) -> Result<Note, String> {
        let mut inner = self.lock();
        let note = Note {
            id: inner.next_id,
            player: player.to_owned(),
            text: text.to_owned(),
            watch,
            by: by.to_owned(),
            at_ms: now_ms(),
        };
        self.log(&mut inner, Logged::Added { note: note.clone() })?;
        Ok(note)
    }

    /// Removes note `id`: its author may, and so may anyone when `any`.
    pub fn remove(&self, id: u64, by: &str, any: bool) -> Result<Note, String> {
        let mut inner = self.lock();
        let note = inner
            .notes
            .iter()
            .find(|n| n.id == id)
            .cloned()
            .ok_or("That note is already gone.")?;
        if !any && name_key(&note.by) != name_key(by) {
            return Err("Only its author or an admin can remove a note.".into());
        }
        self.log(
            &mut inner,
            Logged::Removed {
                id,
                by: by.to_owned(),
                at_ms: now_ms(),
            },
        )?;
        Ok(note)
    }

    /// The notes on `player`, newest first.
    pub fn of(&self, player: &str) -> Vec<Note> {
        let key = name_key(player);
        let inner = self.lock();
        inner
            .notes
            .iter()
            .rev()
            .filter(|n| name_key(&n.player) == key)
            .cloned()
            .collect()
    }

    /// The players on the watchlist, each with the newest note putting them
    /// there, most recently watched first.
    pub fn watched(&self) -> Vec<Note> {
        let inner = self.lock();
        let mut seen = std::collections::HashSet::new();
        inner
            .notes
            .iter()
            .rev()
            .filter(|n| n.watch && seen.insert(name_key(&n.player)))
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_rebuild_and_only_their_author_or_an_admin_removes_them() {
        let path = std::env::temp_dir().join(format!("rs-admin-notes-{}.jsonl", now_ms()));
        let notes = Notes::open(path.clone());
        let first = notes
            .add("Ash", "Bob", "Asks for gold a lot", false)
            .unwrap();
        let watch = notes.add("Kim", "bob", "Watch the GE", true).unwrap();
        notes.add("Kim", "Eve", "Second account?", true).unwrap();
        notes.add("Kim", "Eve", "Seen with Bob", true).unwrap();
        assert_eq!(notes.of("BOB").len(), 2);
        assert_eq!(notes.of("Bob")[0], watch);

        // One entry per watched player: their newest watch note.
        let watched: Vec<(String, String)> = notes
            .watched()
            .into_iter()
            .map(|n| (n.player, n.text))
            .collect();
        assert_eq!(
            watched,
            [
                ("Eve".to_owned(), "Seen with Bob".to_owned()),
                ("bob".to_owned(), "Watch the GE".to_owned())
            ]
        );

        assert!(notes.remove(first.id, "Kim", false).is_err());
        notes.remove(watch.id, "kim", false).unwrap();
        notes.remove(first.id, "Lee", true).unwrap();
        assert!(notes.remove(first.id, "Lee", true).is_err());

        let reopened = Notes::open(path.clone());
        assert!(reopened.of("Bob").is_empty());
        assert_eq!(reopened.watched().len(), 1);
        // Ids carry on from the log.
        assert_eq!(reopened.add("Ash", "Zed", "x", false).unwrap().id, 5);
        let _ = std::fs::remove_file(path);
    }
}
