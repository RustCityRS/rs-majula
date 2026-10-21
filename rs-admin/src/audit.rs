use crate::users::now_ms;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::{error, info, warn};

/// How many of the newest entries are kept in memory; older pages read
/// the file.
const CACHED: usize = 5000;
/// How many entries an older page reads from the file at a time.
const BLOCK: usize = 512;

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

impl Entry {
    /// Whose entry it is: the staff member who acted, or for a sign-in
    /// nobody got through (one with an address but no actor), the account
    /// it named. Entries the console made itself have neither.
    pub fn owner(&self) -> Option<&str> {
        self.actor
            .as_deref()
            .or(self.ip.and(self.target.as_deref()))
    }
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

    /// Something the console did on its own, on `actor`'s behalf (a
    /// scheduled job), from no address.
    pub fn system(actor: Option<&str>, action: &'static str) -> Self {
        Self {
            actor: actor.map(str::to_owned),
            ip: None,
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
    /// Where each readable entry's line starts in the file: entry `id` is
    /// at `offsets[id - 1]`. Ids are the entries' order in the file, so
    /// they stay the same across restarts.
    offsets: Vec<u64>,
    /// The file's length: where the next line goes.
    end: u64,
    /// The newest entries, so the first pages don't touch the file.
    recent: VecDeque<Entry>,
    /// How many `recent` holds at most.
    cache: usize,
}

impl Inner {
    /// The id of the oldest entry in `recent`.
    fn first_cached(&self) -> u64 {
        self.offsets.len() as u64 - self.recent.len() as u64 + 1
    }

    /// The byte range holding entries `lo..=hi`.
    fn span(&self, lo: u64, hi: u64) -> (u64, u64) {
        let start = self.offsets[lo as usize - 1];
        let end = self.offsets.get(hi as usize).copied().unwrap_or(self.end);
        (start, end)
    }
}

/// The audit trail: every entry is appended to a JSON-lines file and synced
/// to disk before it counts, and all of it can be paged and searched, the
/// oldest included.
pub struct Audit {
    path: PathBuf,
    inner: Mutex<Inner>,
}

impl Audit {
    /// Opens the trail at `path`, indexing every entry in it.
    pub fn open(path: PathBuf) -> Self {
        Self::with_cache(path, CACHED)
    }

    fn with_cache(path: PathBuf, cache: usize) -> Self {
        let mut inner = Inner {
            offsets: Vec::new(),
            end: 0,
            recent: VecDeque::with_capacity(cache),
            cache,
        };
        match std::fs::File::open(&path) {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let mut line = Vec::new();
                // Whether the last line read ended in a newline.
                let mut ended = true;
                loop {
                    line.clear();
                    let at = inner.end;
                    match reader.read_until(b'\n', &mut line) {
                        Ok(0) => break,
                        Ok(n) => {
                            inner.end += n as u64;
                            ended = line.last() == Some(&b'\n');
                        }
                        Err(e) => {
                            error!("Stopped reading the audit log {}: {}", path.display(), e);
                            break;
                        }
                    }
                    let text = line.trim_ascii();
                    if text.is_empty() {
                        continue;
                    }
                    match serde_json::from_slice::<Entry>(text) {
                        Ok(mut entry) => {
                            inner.offsets.push(at);
                            entry.id = inner.offsets.len() as u64;
                            if inner.recent.len() == inner.cache {
                                inner.recent.pop_front();
                            }
                            inner.recent.push_back(entry);
                        }
                        Err(e) => warn!("Skipping an unreadable audit entry: {}", e),
                    }
                }
                // A line a crash cut short is ended, so the next entry
                // starts a line of its own rather than joining it.
                if !ended {
                    match append_bytes(&path, b"\n") {
                        Ok(()) => inner.end += 1,
                        Err(e) => error!("Couldn't end the audit log's last line: {}", e),
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => error!("Couldn't open the audit log {}: {}", path.display(), e),
        }
        info!(
            "Audit log: {} entries in {}",
            inner.offsets.len(),
            path.display()
        );
        Self {
            path,
            inner: Mutex::new(inner),
        }
    }

    pub fn record(&self, event: Event) {
        let mut entry = Entry {
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

        let mut line = serde_json::to_vec(&entry).expect("entries serialise");
        line.push(b'\n');
        let mut inner = self.lock();
        // Only an entry on disk gets an id: the trail never shows what a
        // restart would lose.
        if let Err(e) = append_bytes(&self.path, &line) {
            error!(
                "Failed to write the audit log {}; this entry is only in the console's log: {}",
                self.path.display(),
                e
            );
            return;
        }
        let at = inner.end;
        inner.offsets.push(at);
        inner.end += line.len() as u64;
        entry.id = inner.offsets.len() as u64;
        if inner.recent.len() == inner.cache {
            inner.recent.pop_front();
        }
        inner.recent.push_back(entry);
    }

    /// Up to `limit` entries older than `before`, newest first, that pass
    /// `keep`. Pages past the newest entries read the file, a block at a
    /// time, without holding up new entries.
    pub fn page(
        &self,
        before: Option<u64>,
        limit: usize,
        keep: impl Fn(&Entry) -> bool,
    ) -> Vec<Entry> {
        let mut out = Vec::new();
        // The newest id still to look at; 0 when there's none.
        let mut next = {
            let inner = self.lock();
            let last = inner.offsets.len() as u64;
            let top = before.map_or(last, |b| b.saturating_sub(1).min(last));
            let first_cached = inner.first_cached();
            out.extend(
                inner
                    .recent
                    .iter()
                    .rev()
                    .filter(|e| e.id <= top)
                    .filter(|e| keep(e))
                    .take(limit)
                    .cloned(),
            );
            top.min(first_cached.saturating_sub(1))
        };
        while out.len() < limit && next > 0 {
            let lo = next.saturating_sub(BLOCK as u64 - 1).max(1);
            match self.read_block(lo, next) {
                Ok(block) => out.extend(
                    block
                        .into_iter()
                        .rev()
                        .filter(|e| keep(e))
                        .take(limit - out.len()),
                ),
                Err(e) => {
                    warn!(
                        "Failed to read the audit log {}: {}",
                        self.path.display(),
                        e
                    );
                    break;
                }
            }
            next = lo - 1;
        }
        out
    }

    /// Up to `limit` of the newest entries kept in memory that pass `keep`,
    /// newest first: a quick search that never reads the file.
    pub fn recent(&self, limit: usize, keep: impl Fn(&Entry) -> bool) -> Vec<Entry> {
        self.lock()
            .recent
            .iter()
            .rev()
            .filter(|e| keep(e))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Every entry from `since_ms` on, newest first, that passes `keep`.
    pub fn since(&self, since_ms: u64, keep: impl Fn(&Entry) -> bool) -> Vec<Entry> {
        let mut out = Vec::new();
        let mut before = None;
        loop {
            let page = self.page(before, BLOCK, |_| true);
            let Some(last) = page.last() else { break };
            before = Some(last.id);
            let done = last.at_ms < since_ms;
            out.extend(page.into_iter().filter(|e| e.at_ms >= since_ms && keep(e)));
            if done {
                break;
            }
        }
        out
    }

    /// Entries `lo..=hi`, oldest first, from the file.
    fn read_block(&self, lo: u64, hi: u64) -> std::io::Result<Vec<Entry>> {
        let (starts, (from, to)) = {
            let inner = self.lock();
            let starts: Vec<u64> = inner.offsets[lo as usize - 1..hi as usize].to_vec();
            (starts, inner.span(lo, hi))
        };
        let mut file = std::fs::File::open(&self.path)?;
        file.seek(SeekFrom::Start(from))?;
        let mut bytes = vec![0; (to - from) as usize];
        file.read_exact(&mut bytes)?;
        Ok(starts
            .iter()
            .zip(lo..)
            .filter_map(|(&start, id)| {
                let rest = &bytes[(start - from) as usize..];
                let line = rest.split(|&b| b == b'\n').next().unwrap_or_default();
                let mut entry: Entry = serde_json::from_slice(line.trim_ascii()).ok()?;
                entry.id = id;
                Some(entry)
            })
            .collect())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Appends `bytes` to `path` and syncs them to disk.
fn append_bytes(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_data()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn failed_sign_ins_belong_to_the_account_they_named() {
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let entry = |event: Event| Entry {
            id: 0,
            at_ms: 0,
            actor: event.actor,
            ip: event.ip,
            action: event.action.into(),
            target: event.target,
            detail: event.detail,
            ok: event.ok,
        };
        let kick = entry(Event::new(Some("ash"), ip, "kick").target("p1"));
        assert_eq!(kick.owner(), Some("ash"));
        let failed = entry(Event::new(None, ip, "sign_in_failed").target("ash"));
        assert_eq!(failed.owner(), Some("ash"));
        let alert = entry(Event::system(None, "economy_alert").target("p1"));
        assert_eq!(alert.owner(), None);
    }

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

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("rs-admin-audit-{name}-{}.jsonl", now_ms()))
    }

    fn targets(entries: &[Entry]) -> Vec<(u64, String)> {
        entries
            .iter()
            .map(|e| (e.id, e.target.clone().unwrap_or_default()))
            .collect()
    }

    #[test]
    fn the_whole_trail_pages_and_searches_past_the_cache() {
        let path = temp_path("all");
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        // Only 3 kept in memory, and blocks smaller than the trail.
        let audit = Audit::with_cache(path.clone(), 3);
        for i in 1..=1200 {
            let action = if i % 100 == 0 { "ban" } else { "kick" };
            audit.record(Event::new(Some("ash"), ip, action).target(format!("p{i}")));
        }
        for audit in [audit, Audit::with_cache(path.clone(), 3)] {
            // Ids are the order in the file, before a restart and after.
            let mut all = Vec::new();
            let mut before = None;
            loop {
                let page = audit.page(before, 50, |_| true);
                let Some(last) = page.last() else { break };
                before = Some(last.id);
                all.extend(page);
            }
            let expected: Vec<(u64, String)> =
                (1..=1200).rev().map(|i| (i, format!("p{i}"))).collect();
            assert_eq!(targets(&all), expected);

            // A search reaches back through the file, across blocks.
            let bans = audit.page(None, 100, |e| e.action == "ban");
            let expected: Vec<(u64, String)> = (1..=12)
                .rev()
                .map(|i| (i * 100, format!("p{}", i * 100)))
                .collect();
            assert_eq!(targets(&bans), expected);
            let older = audit.page(Some(600), 2, |e| e.action == "ban");
            assert_eq!(
                targets(&older),
                [(500, "p500".into()), (400, "p400".into())]
            );
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn a_line_cut_short_by_a_crash_is_set_aside() {
        let path = temp_path("torn");
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        Audit::open(path.clone()).record(Event::new(Some("ash"), ip, "kick").target("p1"));
        // A crash part way through the next entry.
        append_bytes(&path, b"{\"at_ms\":1,\"actor\":\"as").unwrap();

        let audit = Audit::open(path.clone());
        audit.record(Event::new(Some("ash"), ip, "kick").target("p2"));
        let reopened = Audit::with_cache(path.clone(), 1);
        assert_eq!(
            targets(&reopened.page(None, 10, |_| true)),
            [(2, "p2".into()), (1, "p1".into())]
        );
        let _ = std::fs::remove_file(path);
    }
}
