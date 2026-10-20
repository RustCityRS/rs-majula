use crate::users::now_ms;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::Write;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::{info, warn};

/// How many entries the console can page back through.
const KEEP: usize = 5000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    #[serde(default, skip_serializing_if = "is_zero")]
    pub id: u64,
    pub at_ms: u64,
    pub actor: Option<String>,
    pub ip: Option<IpAddr>,
    pub action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
    pub ok: bool,
}

fn is_zero(n: &u64) -> bool {
    *n == 0
}

/// One audit event, built up then recorded with [`Audit::record`].
pub struct Event {
    actor: Option<String>,
    ip: Option<IpAddr>,
    action: &'static str,
    target: Option<String>,
    detail: String,
    ok: bool,
}

impl Event {
    pub fn new(actor: Option<&str>, ip: IpAddr, action: &'static str) -> Self {
        Self {
            actor: actor.map(str::to_owned),
            ip: Some(ip),
            action,
            target: None,
            detail: String::new(),
            ok: true,
        }
    }

    pub fn target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }

    pub fn ok(mut self, ok: bool) -> Self {
        self.ok = ok;
        self
    }
}

struct Inner {
    recent: VecDeque<Entry>,
    next_id: u64,
}

pub struct Audit {
    path: PathBuf,
    inner: Mutex<Inner>,
}

impl Audit {
    /// Opens the trail at `path`, loading its newest entries.
    pub fn open(path: PathBuf) -> Self {
        let mut recent = VecDeque::with_capacity(KEEP);
        if let Ok(text) = std::fs::read_to_string(&path) {
            for line in text.lines().filter(|l| !l.trim().is_empty()) {
                match serde_json::from_str::<Entry>(line) {
                    Ok(entry) => {
                        if recent.len() == KEEP {
                            recent.pop_front();
                        }
                        recent.push_back(entry);
                    }
                    Err(e) => warn!("Skipping an unreadable audit entry: {}", e),
                }
            }
        }
        let mut next_id = 1;
        for entry in &mut recent {
            entry.id = next_id;
            next_id += 1;
        }
        Self {
            path,
            inner: Mutex::new(Inner { recent, next_id }),
        }
    }

    pub fn record(&self, event: Event) {
        let entry = Entry {
            id: 0,
            at_ms: now_ms(),
            actor: event.actor,
            ip: event.ip,
            action: event.action.to_owned(),
            target: event.target,
            detail: event.detail,
            ok: event.ok,
        };
        info!(
            target: "audit",
            actor = entry.actor.as_deref().unwrap_or("-"),
            ip = %entry.ip.map_or_else(String::new, |ip| ip.to_string()),
            action = %entry.action,
            target_name = entry.target.as_deref().unwrap_or(""),
            ok = entry.ok,
            "{}",
            entry.detail
        );

        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Err(e) = self.append(&entry) {
            warn!(
                "Failed to write the audit log {}: {}",
                self.path.display(),
                e
            );
        }
        let mut entry = entry;
        entry.id = inner.next_id;
        inner.next_id += 1;
        if inner.recent.len() == KEEP {
            inner.recent.pop_front();
        }
        inner.recent.push_back(entry);
    }

    fn append(&self, entry: &Entry) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        let mut line = serde_json::to_vec(entry)?;
        line.push(b'\n');
        file.write_all(&line)
    }

    /// Up to `limit` entries older than `before`, newest first, that pass
    /// `keep`.
    pub fn page(
        &self,
        before: Option<u64>,
        limit: usize,
        keep: impl Fn(&Entry) -> bool,
    ) -> Vec<Entry> {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner
            .recent
            .iter()
            .rev()
            .filter(|e| before.is_none_or(|b| e.id < b))
            .filter(|e| keep(e))
            .take(limit)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn entries_survive_a_restart_and_page_newest_first() {
        let path = std::env::temp_dir().join(format!("rs-admin-audit-{}.jsonl", now_ms()));
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let audit = Audit::open(path.clone());
        for i in 0..5 {
            audit.record(Event::new(Some("ash"), ip, "kick").target(format!("p{i}")));
        }
        let reopened = Audit::open(path.clone());
        let page = reopened.page(None, 2, |_| true);
        assert_eq!(page.len(), 2);
        assert_eq!(page[0].target.as_deref(), Some("p4"));
        let older = reopened.page(Some(page[1].id), 10, |_| true);
        assert_eq!(older.len(), 3);
        assert_eq!(older[0].target.as_deref(), Some("p2"));
        let _ = std::fs::remove_file(path);
    }
}
