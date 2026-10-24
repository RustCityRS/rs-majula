use crate::users::now_ms;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Condvar, Mutex};
use tracing::{error, info, warn};

/// How many of the newest entries are kept in memory; older pages read
/// the file.
const CACHED: usize = 5000;
/// How many entries an older page reads from the file at a time.
const BLOCK: usize = 512;
/// Lookups of a player by one staff member this close together, like a
/// profile loading its timeline, chat and addresses, make one entry.
pub const GATHER_MS: u64 = 2_000;
/// How long after a lookup is recorded the same one again isn't.
const REPEAT_MS: u64 = 15 * 60 * 1000;
/// The lookups gathered, and what each one shows, in the order an entry
/// naming several lists them.
const LOOKUPS: [(&str, &str); 3] = [
    ("timeline_viewed", "timeline"),
    ("addresses_viewed", "login addresses"),
    ("chat_viewed", "chat logs"),
];
/// An entry for several of [`LOOKUPS`] at once; its detail lists them.
const RECORDS_VIEWED: &str = "records_viewed";

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

/// One staff member's lookups of one target.
#[derive(Default)]
struct Lookups {
    ip: Option<IpAddr>,
    /// When the first of `pending` was made.
    since_ms: u64,
    /// Made but in no entry yet.
    pending: Vec<&'static str>,
    /// Recorded, and when.
    recorded: Vec<(&'static str, u64)>,
}

/// How many entries have been written (or failed to be), for waiting on.
#[derive(Default)]
struct Written {
    count: Mutex<u64>,
    changed: Condvar,
}

/// The audit trail: every entry is appended to a JSON-lines file and synced
/// to disk before it counts, and all of it can be paged and searched, the
/// oldest included. A thread of its own writes the entries, those waiting
/// together, so recording one never waits on the disk.
pub struct Audit {
    path: PathBuf,
    inner: Arc<Mutex<Inner>>,
    /// Entries for the writer, with their lines.
    queue: Sender<(Entry, Vec<u8>)>,
    queued: AtomicU64,
    written: Arc<Written>,
    /// By actor and target.
    lookups: Mutex<HashMap<(String, String), Lookups>>,
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
        let inner = Arc::new(Mutex::new(inner));
        let written = Arc::new(Written::default());
        let (queue, entries) = mpsc::channel();
        let writer = (path.clone(), inner.clone(), written.clone());
        std::thread::Builder::new()
            .name("audit".into())
            .spawn(move || keep_writing(writer.0, &writer.1, &writer.2, &entries))
            .expect("starting the audit writer");
        Self {
            path,
            inner,
            queue,
            queued: AtomicU64::new(0),
            written,
            lookups: Mutex::new(HashMap::new()),
        }
    }

    /// Waits for every entry recorded so far to be written.
    pub fn flush(&self) {
        let queued = self.queued.load(Ordering::Acquire);
        let mut count = self.written.count.lock().unwrap_or_else(|e| e.into_inner());
        while *count < queued {
            count = self
                .written
                .changed
                .wait(count)
                .unwrap_or_else(|e| e.into_inner());
        }
    }

    pub fn record(&self, event: Event) {
        self.write(Entry {
            id: 0,
            at_ms: now_ms(),
            actor: event.actor,
            ip: event.ip,
            action: event.action.to_owned(),
            target: event.target,
            detail: event.detail,
            ok: event.ok,
        });
    }

    /// Records a look at a player's records, one of [`LOOKUPS`]: those
    /// made within [`GATHER_MS`] of each other share an entry, written by
    /// [`Audit::write_lookups`], and one already recorded in the last
    /// [`REPEAT_MS`] isn't again.
    pub fn record_lookup(&self, event: Event) {
        let (Some(actor), Some(target)) = (event.actor, event.target) else {
            return;
        };
        debug_assert!(LOOKUPS.iter().any(|(a, _)| *a == event.action));
        let now = now_ms();
        let mut lookups = self.lookups.lock().unwrap_or_else(|e| e.into_inner());
        let l = lookups.entry((actor, target)).or_default();
        let recent = l
            .recorded
            .iter()
            .any(|&(a, at)| a == event.action && now.saturating_sub(at) < REPEAT_MS);
        if recent || l.pending.contains(&event.action) {
            return;
        }
        if l.pending.is_empty() {
            l.since_ms = now;
            l.ip = event.ip;
        }
        l.pending.push(event.action);
    }

    /// Writes the entries for lookups gathered for [`GATHER_MS`] (or all of
    /// them, when `all`), and forgets those too old to hold up a repeat.
    pub fn write_lookups(&self, all: bool) {
        let now = now_ms();
        let mut due = Vec::new();
        {
            let mut lookups = self.lookups.lock().unwrap_or_else(|e| e.into_inner());
            lookups.retain(|(actor, target), l| {
                if !l.pending.is_empty() && (all || now.saturating_sub(l.since_ms) >= GATHER_MS) {
                    let mut pending = std::mem::take(&mut l.pending);
                    pending.sort_by_key(|a| LOOKUPS.iter().position(|(b, _)| a == b));
                    let (action, detail) = match pending[..] {
                        [one] => (one, String::new()),
                        _ => (
                            RECORDS_VIEWED,
                            pending
                                .iter()
                                .filter_map(|a| LOOKUPS.iter().find(|(b, _)| a == b))
                                .map(|(_, shows)| *shows)
                                .collect::<Vec<_>>()
                                .join(", "),
                        ),
                    };
                    due.push(Entry {
                        id: 0,
                        at_ms: l.since_ms,
                        actor: Some(actor.clone()),
                        ip: l.ip,
                        action: action.to_owned(),
                        target: Some(target.clone()),
                        detail,
                        ok: true,
                    });
                    l.recorded.retain(|(a, _)| !pending.contains(a));
                    l.recorded
                        .extend(pending.into_iter().map(|a| (a, l.since_ms)));
                }
                l.recorded
                    .retain(|&(_, at)| now.saturating_sub(at) < REPEAT_MS);
                !l.pending.is_empty() || !l.recorded.is_empty()
            });
        }
        for entry in due {
            self.write(entry);
        }
    }

    fn write(&self, entry: Entry) {
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
        self.queued.fetch_add(1, Ordering::AcqRel);
        if self.queue.send((entry, line)).is_err() {
            error!("The audit writer stopped; this entry is only in the console's log");
            let mut count = self.written.count.lock().unwrap_or_else(|e| e.into_inner());
            *count += 1;
        }
    }

    /// Up to `limit` entries older than `before`, newest first, that pass
    /// `keep`, once those recorded so far are written. Pages past the
    /// newest entries read the file, a block at a time, without holding up
    /// new entries.
    pub fn page(
        &self,
        before: Option<u64>,
        limit: usize,
        keep: impl Fn(&Entry) -> bool,
    ) -> Vec<Entry> {
        self.flush();
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

impl Drop for Audit {
    fn drop(&mut self) {
        self.flush();
    }
}

/// Writes the entries `queue` brings until the trail is dropped: all those
/// waiting at once, with one sync to disk. Only an entry on disk gets an
/// id, so the trail never shows what a restart would lose.
fn keep_writing(
    path: PathBuf,
    inner: &Mutex<Inner>,
    written: &Written,
    queue: &Receiver<(Entry, Vec<u8>)>,
) {
    while let Ok(first) = queue.recv() {
        let mut batch = vec![first];
        batch.extend(queue.try_iter());
        let n = batch.len() as u64;
        let bytes: Vec<u8> = batch.iter().flat_map(|(_, line)| line).copied().collect();
        match append_bytes(&path, &bytes) {
            Ok(()) => {
                let mut inner = inner.lock().unwrap_or_else(|e| e.into_inner());
                for (mut entry, line) in batch {
                    let at = inner.end;
                    inner.offsets.push(at);
                    inner.end += line.len() as u64;
                    entry.id = inner.offsets.len() as u64;
                    if inner.recent.len() == inner.cache {
                        inner.recent.pop_front();
                    }
                    inner.recent.push_back(entry);
                }
            }
            Err(e) => error!(
                "Failed to write the audit log {}; {} entries are only in the console's log: {}",
                path.display(),
                n,
                e
            ),
        }
        let mut count = written.count.lock().unwrap_or_else(|e| e.into_inner());
        *count += n;
        written.changed.notify_all();
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
    fn lookups_made_together_share_one_entry_and_repeats_none() {
        let path = std::env::temp_dir().join(format!("rs-console-lookups-{}.jsonl", now_ms()));
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let audit = Audit::open(path.clone());
        let look = |action, target: &str| {
            audit.record_lookup(Event::new(Some("ash"), ip, action).target(target));
        };
        look("chat_viewed", "p1");
        look("timeline_viewed", "p1");
        look("addresses_viewed", "p1");
        look("timeline_viewed", "p2");
        // Not yet gathered for long enough.
        audit.write_lookups(false);
        assert!(audit.page(None, 10, |_| true).is_empty());
        audit.write_lookups(true);
        look("chat_viewed", "p1");
        look("timeline_viewed", "p2");
        audit.write_lookups(true);
        let mut entries = audit.page(None, 10, |_| true);
        entries.sort_by(|a, b| a.target.cmp(&b.target));
        let got: Vec<_> = entries
            .iter()
            .map(|e| (e.target.as_deref(), e.action.as_str(), e.detail.as_str()))
            .collect();
        assert_eq!(
            got,
            [
                (
                    Some("p1"),
                    RECORDS_VIEWED,
                    "timeline, login addresses, chat logs"
                ),
                (Some("p2"), "timeline_viewed", ""),
            ]
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn entries_survive_a_restart_and_page_newest_first() {
        let path = std::env::temp_dir().join(format!("rs-console-audit-{}.jsonl", now_ms()));
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let audit = Audit::open(path.clone());
        for i in 0..5 {
            audit.record(Event::new(Some("ash"), ip, "kick").target(format!("p{i}")));
        }
        // Closing it writes what's waiting.
        drop(audit);
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
        std::env::temp_dir().join(format!("rs-console-audit-{name}-{}.jsonl", now_ms()))
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
        audit.flush();
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
        audit.flush();
        let reopened = Audit::with_cache(path.clone(), 1);
        assert_eq!(
            targets(&reopened.page(None, 10, |_| true)),
            [(2, "p2".into()), (1, "p1".into())]
        );
        let _ = std::fs::remove_file(path);
    }
}
