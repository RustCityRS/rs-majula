#[cfg(since_647)]
use crate::pack::js5::PreparedGroup;
use crate::pack::pack_registry::PackedFile;
use crate::pack::packed_data::PackedData;
use anyhow::{Context, Result, ensure};
#[cfg(since_647)]
use rayon::prelude::*;
use rs_io::Packet;
#[cfg(since_647)]
use rs_io::js5::reference::FileInfo;
use rustc_hash::FxHashMap;
use siphasher::sip128::{Hasher128, SipHasher13};
use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::convert::Infallible;
use std::ffi::OsString;
use std::fmt::Display;
use std::fs::{self, File, Metadata, OpenOptions};
use std::hash::Hasher;
use std::io::{self, Write};
use std::mem::take;
#[cfg(unix)]
use std::os::unix::fs::FileExt;
#[cfg(windows)]
use std::os::windows::fs::FileExt;
use std::path::{Path, PathBuf};
#[cfg(since_647)]
use std::slice;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

pub type Key = u128;

const MAGIC: i32 = 0x5253_5043;
const FORMAT: u8 = 2;
const SHARDS: usize = 16;
/// A source changed this close to a pack's start could change again within
/// its timestamp's resolution, so its unit is packed again next time. Well
/// above NTFS's and ext4's resolution, and below the hot reload's 300 ms wait.
const RACY: Duration = Duration::from_millis(100);

static ACTIVE: RwLock<Option<Arc<Store>>> = RwLock::new(None);

/// Identifies the inputs or the output of a step.
pub struct Hash(SipHasher13);

impl Hash {
    pub fn new(domain: &str) -> Self {
        let mut hash = Self(SipHasher13::new());
        hash.bytes(domain.as_bytes());
        hash
    }

    pub fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        self.0.write_u64(bytes.len() as u64);
        self.0.write(bytes);
        self
    }

    pub fn int(&mut self, value: u64) -> &mut Self {
        self.0.write_u64(value);
        self
    }

    pub fn key(&self) -> Key {
        self.0.finish128().as_u128()
    }
}

fn content_key(bytes: &[u8]) -> Key {
    Hash::new("content").bytes(bytes).key()
}

fn fingerprint() -> Key {
    Hash::new("packer")
        .bytes(env!("RS_PACK_FINGERPRINT").as_bytes())
        .bytes(env!("REV").as_bytes())
        .key()
}

fn nanos(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_nanos() as u64)
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn active() -> Option<Arc<Store>> {
    ACTIVE
        .read()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

fn install(store: Option<Arc<Store>>) {
    *ACTIVE.write().unwrap_or_else(PoisonError::into_inner) = store;
}

#[derive(Clone, Copy)]
struct Loc {
    offset: u64,
    len: u32,
}

#[derive(Clone, Copy)]
struct Stat {
    len: u64,
    mtime: u64,
}

impl Stat {
    fn of(meta: &Metadata) -> Self {
        Self {
            len: meta.len(),
            mtime: meta.modified().map_or(0, nanos),
        }
    }
}

type Listing = HashMap<OsString, Stat>;

#[derive(Clone)]
struct Unit {
    sources: Key,
    lookups: Vec<u8>,
    payload: Vec<u8>,
}

#[derive(Default)]
struct StageRecord {
    deps: Vec<(String, Key)>,
    units: HashMap<u32, Unit>,
}

#[derive(Default)]
struct Manifest {
    generation: u32,
    blobs: HashMap<Key, Loc>,
    memo: HashMap<Key, Key>,
    stages: HashMap<String, StageRecord>,
}

struct Store {
    dir: PathBuf,
    started: u64,
    old: Manifest,
    reader: File,
    writer: Mutex<Option<(File, u64)>>,
    blobs: [Mutex<HashMap<Key, Loc>>; SHARDS],
    memo: Mutex<HashMap<Key, Key>>,
    stages: Mutex<HashMap<String, StageRecord>>,
    listings: Mutex<HashMap<PathBuf, Arc<OnceLock<Listing>>>>,
    reused: AtomicUsize,
    packed: AtomicUsize,
    _lock: File,
}

/// A pack's use of the cache in a directory: what an earlier pack made from
/// inputs that haven't changed since is reused, and what this pack makes is
/// kept for the next.
pub struct Session(Option<Arc<Store>>);

pub fn open(dir: &Path) -> Result<Session> {
    fs::create_dir_all(dir).with_context(|| dir.display().to_string())?;
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("lock"))?;
    if lock_file.try_lock().is_err() {
        info!("Waiting for another pack to finish with {}", dir.display());
        lock_file.lock()?;
    }
    let started = nanos(SystemTime::now());
    let old = read_manifest(dir).unwrap_or_else(|e| {
        info!("Pack cache starts over: {e:#}");
        clear(dir);
        Manifest::default()
    });
    let path = blob_path(dir, old.generation);
    let writer = OpenOptions::new().create(true).append(true).open(&path)?;
    let end = writer.metadata()?.len();
    let store = Arc::new(Store {
        dir: dir.to_path_buf(),
        started,
        reader: File::open(&path)?,
        writer: Mutex::new(Some((writer, end))),
        blobs: Default::default(),
        memo: Mutex::default(),
        stages: Mutex::default(),
        listings: Mutex::default(),
        reused: AtomicUsize::new(0),
        packed: AtomicUsize::new(0),
        old,
        _lock: lock_file,
    });
    install(Some(store.clone()));
    Ok(Session(Some(store)))
}

impl Session {
    /// Keeps what this pack made for the next one.
    pub fn finish(mut self) -> Result<()> {
        let Some(store) = self.0.take() else {
            return Ok(());
        };
        install(None);
        store.save()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if self.0.take().is_some() {
            install(None);
        }
    }
}

/// What `compute` makes from the inputs `key` names, from an earlier pack
/// when one made it.
pub fn memo(key: Key, compute: impl FnOnce() -> Result<Vec<u8>>) -> Result<Vec<u8>> {
    memoize(key, compute)
}

/// [`memo`] for a step that can't fail.
pub fn remember(key: Key, compute: impl FnOnce() -> Vec<u8>) -> Vec<u8> {
    let Ok(bytes) = memoize(key, || Ok::<_, Infallible>(compute()));
    bytes
}

fn memoize<E>(key: Key, compute: impl FnOnce() -> Result<Vec<u8>, E>) -> Result<Vec<u8>, E> {
    let Some(store) = active() else {
        return compute();
    };
    if let Some(bytes) = store.memo_get(key) {
        return Ok(bytes);
    }
    let bytes = compute()?;
    let content = content_key(&bytes);
    if store.put(content, &bytes) {
        lock(&store.memo).insert(key, content);
    }
    Ok(bytes)
}

/// The files under `dirs`, in a stable order.
pub fn files_under(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for dir in dirs {
        collect(dir, &mut files);
    }
    files.sort();
    files
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            collect(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// The files as they are now, for a stage that depends on more of them than
/// it would want to read. One changed too recently to trust its timestamp
/// makes a key the next pack won't match.
pub fn stats_key(files: &[PathBuf]) -> Key {
    let store = active();
    let started = store.as_ref().map_or(0, |store| store.started);
    let mut hash = Hash::new("stats");
    for path in files {
        hash.bytes(path.to_string_lossy().as_bytes());
        let stat = match &store {
            Some(store) => store.stat(path),
            None => fs::metadata(path).ok().map(|meta| Stat::of(&meta)),
        };
        if let Some(stat) = stat {
            if stat.mtime + RACY.as_nanos() as u64 >= started {
                hash.int(started);
            }
            hash.int(stat.len).int(stat.mtime);
        }
    }
    hash.key()
}

/// The contents of the files in `dir` (not below it), except `skip`. Only
/// their names count, not where `dir` is.
pub fn dir_key(dir: &Path, skip: &[&str]) -> Key {
    let mut names: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.file_name())
        .filter(|name| name.to_str().is_none_or(|name| !skip.contains(&name)))
        .collect();
    names.sort();
    let mut hash = Hash::new("dir");
    for name in names {
        hash.bytes(name.as_encoded_bytes())
            .bytes(&fs::read(dir.join(&name)).unwrap_or_default());
    }
    hash.key()
}

/// Byte strings as one, for steps that make several.
pub fn bundle(parts: &[&[u8]]) -> Vec<u8> {
    let mut buf = Packet::new(4 + parts.iter().map(|part| 4 + part.len()).sum::<usize>());
    buf.p4(parts.len() as i32);
    for part in parts {
        put_bytes(&mut buf, part);
    }
    buf.data
}

/// The byte strings [`bundle`] made.
pub fn unbundle(bytes: &[u8]) -> Option<Vec<Vec<u8>>> {
    let mut reader = Reader::new(bytes);
    let count = reader.g4().ok()?;
    let mut parts = Vec::new();
    for _ in 0..count {
        parts.push(reader.bytes().ok()?.to_vec());
    }
    Some(parts)
}

/// An output the pack cache can keep.
pub trait Cached: Sized {
    fn encode(&self) -> Vec<u8>;
    fn decode(bytes: Vec<u8>) -> Option<Self>;
}

impl Cached for Vec<u8> {
    fn encode(&self) -> Vec<u8> {
        self.clone()
    }

    fn decode(bytes: Vec<u8>) -> Option<Self> {
        Some(bytes)
    }
}

impl Cached for (Vec<u8>, Vec<u8>) {
    fn encode(&self) -> Vec<u8> {
        bundle(&[&self.0, &self.1])
    }

    fn decode(bytes: Vec<u8>) -> Option<Self> {
        let [first, second] = <[Vec<u8>; 2]>::try_from(unbundle(&bytes)?).ok()?;
        Some((first, second))
    }
}

impl Cached for PackedData {
    fn encode(&self) -> Vec<u8> {
        bundle(&[&self.dat, &self.idx])
    }

    fn decode(bytes: Vec<u8>) -> Option<Self> {
        let (dat, idx) = <(Vec<u8>, Vec<u8>)>::decode(bytes)?;
        Some(PackedData::from_parts(dat, idx))
    }
}

impl Cached for PackedFile {
    fn encode(&self) -> Vec<u8> {
        match &self.client {
            Some(client) => bundle(&[&self.server.dat, &self.server.idx, &client.dat, &client.idx]),
            None => bundle(&[&self.server.dat, &self.server.idx]),
        }
    }

    fn decode(bytes: Vec<u8>) -> Option<Self> {
        let mut parts = unbundle(&bytes)?.into_iter();
        let server = PackedData::from_parts(parts.next()?, parts.next()?);
        let client = match (parts.next(), parts.next()) {
            (Some(dat), Some(idx)) => Some(PackedData::from_parts(dat, idx)),
            (None, None) => None,
            _ => return None,
        };
        Some(PackedFile { server, client })
    }
}

/// The lookups a unit's build made through a registry view, with the answers
/// they got: `(op, kind, query, answer)`. The op, kind and query are kept as
/// one string, the kind ended by a NUL, so a lookup made again, as most are,
/// costs no allocation.
#[derive(Default)]
pub struct Lookups(Mutex<FxHashMap<String, String>>);

thread_local! {
    static LOOKUP: RefCell<String> = const { RefCell::new(String::new()) };
}

impl Lookups {
    /// Records a lookup, with `answer` giving what it got the first time.
    pub fn record(&self, op: u8, kind: &str, query: &str, answer: impl FnOnce() -> String) {
        LOOKUP.with_borrow_mut(|lookup| {
            lookup.clear();
            lookup.push(char::from(op));
            lookup.push_str(kind);
            lookup.push('\0');
            lookup.push_str(query);
            let mut lookups = lock(&self.0);
            if !lookups.contains_key(lookup.as_str()) {
                lookups.insert(lookup.clone(), answer());
            }
        });
    }

    /// The lookups, less any in a kind depended on whole (op `w`), which
    /// cover them.
    pub fn encode(&self) -> Vec<u8> {
        let lookups = lock(&self.0);
        let mut split: Vec<_> = lookups
            .iter()
            .filter_map(|(lookup, answer)| {
                let (kind, query) = lookup.get(1..)?.split_once('\0')?;
                Some((&lookup.as_bytes()[..1], kind, query, answer.as_str()))
            })
            .collect();
        split.sort_unstable();
        let whole: BTreeSet<&str> = split
            .iter()
            .filter(|(op, ..)| *op == b"w")
            .map(|(_, kind, ..)| *kind)
            .collect();
        let mut parts: Vec<&[u8]> = Vec::with_capacity(split.len() << 2);
        for (op, kind, query, answer) in split {
            if op == b"w" || !whole.contains(kind) {
                parts.extend([op, kind.as_bytes(), query.as_bytes(), answer.as_bytes()]);
            }
        }
        bundle(&parts)
    }

    /// The lookups [`encode`](Self::encode) kept.
    pub fn decode(bytes: &[u8]) -> Option<Vec<(u8, String, String, String)>> {
        let parts = unbundle(bytes)?;
        let (lookups, []) = parts.as_chunks::<4>() else {
            return None;
        };
        lookups
            .iter()
            .map(|[op, kind, query, answer]| {
                let [op] = op[..] else {
                    return None;
                };
                let text = |part: &Vec<u8>| String::from_utf8(part.clone()).ok();
                Some((op, text(kind)?, text(query)?, text(answer)?))
            })
            .collect()
    }
}

/// An answer that may be missing, as a lookup records it: `=` before a value
/// tells it apart from none.
pub fn answer(value: Option<impl Display>) -> String {
    value.map_or_else(String::new, |value| format!("={value}"))
}

/// Names a unit's output can depend on. A view records the lookups made
/// through it, and a unit is reused only while they still answer the same.
pub trait Registry: Sync + Sized {
    fn recording(&self) -> Self;
    fn recorded(&self) -> Vec<u8>;
    fn unchanged(&self, recorded: &[u8]) -> bool;
}

/// No names: a unit packed from its sources alone.
impl Registry for () {
    fn recording(&self) -> Self {}

    fn recorded(&self) -> Vec<u8> {
        Vec::new()
    }

    fn unchanged(&self, recorded: &[u8]) -> bool {
        recorded.is_empty()
    }
}

/// One step of a pack. A unit's output is reused while the step's
/// dependencies, the unit's sources and its registry lookups are as an
/// earlier pack recorded them.
pub struct Stage {
    store: Option<Arc<Store>>,
    name: String,
    fresh: bool,
    deps: Vec<(String, Key)>,
    units: Mutex<HashMap<u32, Unit>>,
}

impl Stage {
    /// `deps` are what the step's output depends on besides its units'
    /// sources and lookups.
    pub fn new(name: &str, deps: Vec<(String, Key)>) -> Self {
        let store = active();
        let fresh = store
            .as_ref()
            .and_then(|store| store.old.stages.get(name))
            .is_some_and(|old| {
                old.deps.len() == deps.len() && deps.iter().all(|dep| old.deps.contains(dep))
            });
        Self {
            store,
            name: name.to_owned(),
            fresh,
            deps,
            units: Mutex::default(),
        }
    }

    /// Records the step for the next pack, with `updates` to its
    /// dependencies as the step left them.
    pub fn finish(self, updates: Vec<(String, Key)>) {
        let Some(store) = self.store else {
            return;
        };
        let mut deps = self.deps;
        for (name, key) in updates {
            match deps.iter_mut().find(|(dep, _)| *dep == name) {
                Some(dep) => dep.1 = key,
                None => deps.push((name, key)),
            }
        }
        let units = self
            .units
            .into_inner()
            .unwrap_or_else(PoisonError::into_inner);
        lock(&store.stages).insert(self.name, StageRecord { deps, units });
    }

    /// What `build` makes from `sources`, from an earlier pack when none of
    /// them changed since.
    pub fn unit<T: Cached>(
        &self,
        id: u32,
        sources: &[PathBuf],
        build: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        self.unit_with(&(), id, sources, |_| build())
    }

    /// [`unit`](Self::unit) for a build that looks names up in `registry`.
    pub fn unit_with<R: Registry, T: Cached>(
        &self,
        registry: &R,
        id: u32,
        sources: &[PathBuf],
        build: impl FnOnce(&R) -> Result<T>,
    ) -> Result<T> {
        let Some(store) = &self.store else {
            return build(registry);
        };
        let key = self.sources_key(store, sources);
        if let Some(unit) = self.reusable(registry, id, key)
            && let Some(output) = read_key(&unit.payload)
                .and_then(|content| store.blob(content))
                .and_then(T::decode)
        {
            self.keep(store, id, unit, true);
            return Ok(output);
        }
        let view = registry.recording();
        let output = build(&view)?;
        let bytes = output.encode();
        let content = content_key(&bytes);
        if let Some(sources) = key
            && store.put(content, &bytes)
        {
            let unit = Unit {
                sources,
                lookups: view.recorded(),
                payload: write_key(content),
            };
            self.keep(store, id, unit, false);
        }
        Ok(output)
    }

    fn reusable(&self, registry: &impl Registry, id: u32, sources: Option<Key>) -> Option<Unit> {
        if !self.fresh {
            return None;
        }
        let unit = self
            .store
            .as_ref()?
            .old
            .stages
            .get(&self.name)?
            .units
            .get(&id)?;
        (Some(unit.sources) == sources && registry.unchanged(&unit.lookups)).then(|| unit.clone())
    }

    fn keep(&self, store: &Store, id: u32, unit: Unit, reused: bool) {
        let count = if reused { &store.reused } else { &store.packed };
        count.fetch_add(1, Ordering::Relaxed);
        lock(&self.units).insert(id, unit);
    }

    /// The sources as they are now, or none when one changed too recently
    /// for its timestamp to show a later change.
    fn sources_key(&self, store: &Store, sources: &[PathBuf]) -> Option<Key> {
        let racy = RACY.as_nanos() as u64;
        let mut hash = Hash::new("sources");
        for path in sources {
            hash.bytes(path.to_string_lossy().as_bytes());
            match store.stat(path) {
                Some(stat) => {
                    if stat.mtime + racy >= store.started {
                        return None;
                    }
                    hash.int(1).int(stat.len).int(stat.mtime);
                }
                None => {
                    hash.int(0);
                }
            }
        }
        Some(hash.key())
    }
}

#[cfg(since_647)]
impl Stage {
    /// The groups `build` makes from `items`, each reused from an earlier pack
    /// while its `sources` and the names it looked up in `registry` are
    /// unchanged.
    pub fn groups<R: Registry, T: Sync>(
        &self,
        registry: &R,
        items: &[(u32, T)],
        sources: impl Fn(&T) -> Vec<PathBuf> + Sync,
        build: impl Fn(u32, &T, &R) -> Result<PreparedGroup> + Sync,
    ) -> Result<Vec<PreparedGroup>> {
        let Some(store) = &self.store else {
            return items
                .par_iter()
                .map(|(id, item)| build(*id, item, registry))
                .collect();
        };
        items
            .par_iter()
            .map(|(id, item)| {
                let key = self.sources_key(store, &sources(item));
                if let Some(unit) = self.reusable(registry, *id, key)
                    && let Some(mut groups) = read_groups(store, &unit.payload)
                    && groups.len() == 1
                {
                    self.keep(store, *id, unit, true);
                    return Ok(groups.remove(0));
                }
                let view = registry.recording();
                let group = build(*id, item, &view)?;
                if let Some(sources) = key
                    && let Some(payload) = write_groups(store, slice::from_ref(&group))
                {
                    let unit = Unit {
                        sources,
                        lookups: view.recorded(),
                        payload,
                    };
                    self.keep(store, *id, unit, false);
                }
                Ok(group)
            })
            .collect()
    }

    /// The groups one step makes from `sources`, from an earlier pack when
    /// none of them, and none of the names it looked up, changed since.
    pub fn unit_groups<R: Registry>(
        &self,
        registry: &R,
        id: u32,
        sources: &[PathBuf],
        build: impl FnOnce(&R) -> Result<Vec<PreparedGroup>>,
    ) -> Result<Vec<PreparedGroup>> {
        let Some(store) = &self.store else {
            return build(registry);
        };
        let key = self.sources_key(store, sources);
        if let Some(unit) = self.reusable(registry, id, key)
            && let Some(groups) = read_groups(store, &unit.payload)
        {
            self.keep(store, id, unit, true);
            return Ok(groups);
        }
        let view = registry.recording();
        let groups = build(&view)?;
        if let Some(sources) = key
            && let Some(payload) = write_groups(store, &groups)
        {
            let unit = Unit {
                sources,
                lookups: view.recorded(),
                payload,
            };
            self.keep(store, id, unit, false);
        }
        Ok(groups)
    }
}

/// A group's header, with its container kept as a blob of its own: the memo
/// that made it keeps the same bytes.
#[cfg(since_647)]
fn write_groups(store: &Store, groups: &[PreparedGroup]) -> Option<Vec<u8>> {
    let mut size = 4;
    for group in groups {
        size += 4 * 5 + 1 + 4 + 16;
        size += group.whirlpool.map_or(0, |hash| hash.len());
        size += group
            .files
            .iter()
            .map(|file| 5 + if file.name_hash.is_some() { 4 } else { 0 })
            .sum::<usize>();
    }
    let mut buf = Packet::new(size);
    buf.p4(groups.len() as i32);
    for group in groups {
        let content = content_key(&group.raw);
        if !store.put(content, &group.raw) {
            return None;
        }
        buf.p4(group.id as i32);
        buf.p4(group.version as i32);
        buf.p4(group.size as i32);
        buf.p4(group.crc as i32);
        buf.p4(group.name_hash);
        match &group.whirlpool {
            Some(hash) => {
                buf.p1(1);
                buf.pdata(hash, 0, hash.len());
            }
            None => buf.p1(0),
        }
        buf.p4(group.files.len() as i32);
        for file in &group.files {
            buf.p4(file.id as i32);
            match file.name_hash {
                Some(hash) => {
                    buf.p1(1);
                    buf.p4(hash);
                }
                None => buf.p1(0),
            }
        }
        put_key(&mut buf, content);
    }
    Some(buf.data)
}

#[cfg(since_647)]
fn read_groups(store: &Store, payload: &[u8]) -> Option<Vec<PreparedGroup>> {
    let mut reader = Reader::new(payload);
    let count = reader.g4().ok()?;
    let mut groups = Vec::new();
    for _ in 0..count {
        let id = reader.g4().ok()?;
        let version = reader.g4().ok()?;
        let size = reader.g4().ok()? as usize;
        let crc = reader.g4().ok()?;
        let name_hash = reader.g4().ok()? as i32;
        let whirlpool = match reader.g1().ok()? {
            0 => None,
            _ => {
                let mut hash = [0; 64];
                hash.copy_from_slice(reader.take(64).ok()?);
                Some(hash)
            }
        };
        let mut files = Vec::new();
        for _ in 0..reader.g4().ok()? {
            let id = reader.g4().ok()?;
            let name_hash = match reader.g1().ok()? {
                0 => None,
                _ => Some(reader.g4().ok()? as i32),
            };
            files.push(FileInfo { id, name_hash });
        }
        let raw = store.blob(reader.key().ok()?)?;
        groups.push(PreparedGroup {
            id,
            raw,
            crc,
            version,
            size,
            name_hash,
            whirlpool,
            files,
        });
    }
    Some(groups)
}

fn write_key(key: Key) -> Vec<u8> {
    let mut buf = Packet::new(16);
    put_key(&mut buf, key);
    buf.data
}

fn read_key(payload: &[u8]) -> Option<Key> {
    Reader::new(payload).key().ok()
}

fn put_key(buf: &mut Packet, key: Key) {
    buf.p8((key >> 64) as i64);
    buf.p8(key as i64);
}

fn put_bytes(buf: &mut Packet, bytes: &[u8]) {
    buf.p4(bytes.len() as i32);
    buf.pdata(bytes, 0, bytes.len());
}

/// Packet reads that check the bytes are there first: the cache's files may
/// be cut short or damaged.
struct Reader<'a>(Packet<&'a [u8]>);

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self(Packet::from(bytes))
    }

    fn need(&self, len: usize) -> Result<()> {
        ensure!(
            self.0.data.len() - self.0.pos >= len,
            "Pack cache record cut short"
        );
        Ok(())
    }

    fn g1(&mut self) -> Result<u8> {
        self.need(1)?;
        Ok(self.0.g1())
    }

    fn g4(&mut self) -> Result<u32> {
        self.need(4)?;
        Ok(self.0.g4s() as u32)
    }

    fn g8(&mut self) -> Result<u64> {
        self.need(8)?;
        Ok(self.0.g8s() as u64)
    }

    fn key(&mut self) -> Result<Key> {
        let high = self.g8()? as Key;
        Ok(high << 64 | self.g8()? as Key)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        self.need(len)?;
        let start = self.0.pos;
        self.0.pos += len;
        Ok(&self.0.data[start..start + len])
    }

    fn bytes(&mut self) -> Result<&'a [u8]> {
        let len = self.g4()? as usize;
        self.take(len)
    }

    fn string(&mut self) -> Result<String> {
        Ok(String::from_utf8(self.bytes()?.to_vec())?)
    }
}

impl Store {
    fn blob(&self, key: Key) -> Option<Vec<u8>> {
        let shard = &self.blobs[key as usize & (SHARDS - 1)];
        let live = lock(shard).get(&key).copied();
        let loc = live.or_else(|| self.old.blobs.get(&key).copied())?;
        let mut bytes = vec![0; loc.len as usize];
        read_at(&self.reader, &mut bytes, loc.offset).ok()?;
        if live.is_none() {
            lock(shard).insert(key, loc);
        }
        Some(bytes)
    }

    /// Keeps `bytes` under their `key`, once. False when the cache can't.
    fn put(&self, key: Key, bytes: &[u8]) -> bool {
        let shard = &self.blobs[key as usize & (SHARDS - 1)];
        if lock(shard).contains_key(&key) {
            return true;
        }
        let loc = match self.old.blobs.get(&key) {
            Some(&loc) => loc,
            None => {
                let Ok(len) = u32::try_from(bytes.len()) else {
                    return false;
                };
                let mut writer = lock(&self.writer);
                let Some((file, end)) = writer.as_mut() else {
                    return false;
                };
                if let Err(e) = file.write_all(bytes) {
                    warn!("Pack cache stops keeping outputs: {e}");
                    *writer = None;
                    return false;
                }
                let loc = Loc { offset: *end, len };
                *end += u64::from(len);
                loc
            }
        };
        lock(shard).insert(key, loc);
        true
    }

    /// A file's size and modification time, from a listing of its directory
    /// made once a pack: on Windows a listing has them without opening each
    /// file. One the listing misses is looked up itself, as it may have been
    /// made since or be named in another case.
    fn stat(&self, path: &Path) -> Option<Stat> {
        let dir = match path.parent() {
            Some(dir) if !dir.as_os_str().is_empty() => dir,
            _ => Path::new("."),
        };
        let listing = {
            let mut listings = lock(&self.listings);
            match listings.get(dir) {
                Some(listing) => listing.clone(),
                None => listings.entry(dir.to_path_buf()).or_default().clone(),
            }
        };
        path.file_name()
            .and_then(|name| listing.get_or_init(|| list(dir)).get(name).copied())
            .or_else(|| fs::metadata(path).ok().map(|meta| Stat::of(&meta)))
    }

    fn memo_get(&self, key: Key) -> Option<Vec<u8>> {
        let live = lock(&self.memo).get(&key).copied();
        let content = live.or_else(|| self.old.memo.get(&key).copied())?;
        let bytes = self.blob(content)?;
        if live.is_none() {
            lock(&self.memo).insert(key, content);
        }
        Some(bytes)
    }

    fn save(&self) -> Result<()> {
        let mut blobs: HashMap<Key, Loc> = self
            .blobs
            .iter()
            .flat_map(|shard| take(&mut *lock(shard)))
            .collect();
        let memo: HashMap<Key, Key> = take(&mut *lock(&self.memo))
            .into_iter()
            .filter(|(_, content)| blobs.contains_key(content))
            .collect();
        let stages = take(&mut *lock(&self.stages));
        let mut generation = self.old.generation;
        let end = match take(&mut *lock(&self.writer)) {
            Some((file, end)) => {
                file.sync_all()?;
                end
            }
            None => u64::MAX,
        };
        let live: u64 = blobs.values().map(|loc| u64::from(loc.len)).sum();
        if end > (live << 1) + (64 << 20) {
            generation += 1;
            blobs = self.compact(blobs, generation)?;
        }
        write_manifest(
            &self.dir,
            &Manifest {
                generation,
                blobs,
                memo,
                stages,
            },
        )?;
        if generation != self.old.generation {
            let _ = fs::remove_file(blob_path(&self.dir, self.old.generation));
        }
        info!(
            "Pack cache: {} reused, {} packed",
            self.reused.load(Ordering::Relaxed),
            self.packed.load(Ordering::Relaxed)
        );
        Ok(())
    }

    /// Copies the blobs still in use to a new file, leaving the old outputs
    /// behind.
    fn compact(&self, blobs: HashMap<Key, Loc>, generation: u32) -> Result<HashMap<Key, Loc>> {
        let mut file = File::create(blob_path(&self.dir, generation))?;
        let mut order: Vec<_> = blobs.into_iter().collect();
        order.sort_by_key(|(_, loc)| loc.offset);
        let mut moved = HashMap::with_capacity(order.len());
        let mut offset = 0;
        for (key, loc) in order {
            let mut bytes = vec![0; loc.len as usize];
            read_at(&self.reader, &mut bytes, loc.offset)?;
            file.write_all(&bytes)?;
            moved.insert(
                key,
                Loc {
                    offset,
                    len: loc.len,
                },
            );
            offset += u64::from(loc.len);
        }
        file.sync_all()?;
        Ok(moved)
    }
}

/// The entries of `dir` as they are now. A link stands for what it points
/// to.
fn list(dir: &Path) -> Listing {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            let meta = if meta.is_symlink() {
                fs::metadata(entry.path()).ok()?
            } else {
                meta
            };
            Some((entry.file_name(), Stat::of(&meta)))
        })
        .collect()
}

fn blob_path(dir: &Path, generation: u32) -> PathBuf {
    dir.join(format!("blobs.{generation}"))
}

/// Removes what an earlier pack kept, but not the lock this pack holds.
fn clear(dir: &Path) {
    for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
        if entry.file_name() != "lock" {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn read_manifest(dir: &Path) -> Result<Manifest> {
    let bytes = fs::read(dir.join("manifest")).context("no earlier pack")?;
    let mut reader = Reader::new(&bytes);
    ensure!(
        reader.g4()? as i32 == MAGIC && reader.g1()? == FORMAT,
        "its format changed"
    );
    ensure!(reader.key()? == fingerprint(), "the packer changed");
    let generation = reader.g4()?;
    let stored = fs::metadata(blob_path(dir, generation))
        .context("its outputs are gone")?
        .len();
    let mut manifest = Manifest {
        generation,
        ..Default::default()
    };
    for _ in 0..reader.g4()? {
        let key = reader.key()?;
        let offset = reader.g8()?;
        let len = reader.g4()?;
        if offset + u64::from(len) <= stored {
            manifest.blobs.insert(key, Loc { offset, len });
        }
    }
    for _ in 0..reader.g4()? {
        let key = reader.key()?;
        manifest.memo.insert(key, reader.key()?);
    }
    for _ in 0..reader.g4()? {
        let name = reader.string()?;
        let mut stage = StageRecord::default();
        for _ in 0..reader.g4()? {
            let dep = reader.string()?;
            stage.deps.push((dep, reader.key()?));
        }
        for _ in 0..reader.g4()? {
            let id = reader.g4()?;
            let sources = reader.key()?;
            let lookups = reader.bytes()?.to_vec();
            let payload = reader.bytes()?.to_vec();
            stage.units.insert(
                id,
                Unit {
                    sources,
                    lookups,
                    payload,
                },
            );
        }
        manifest.stages.insert(name, stage);
    }
    Ok(manifest)
}

fn write_manifest(dir: &Path, manifest: &Manifest) -> Result<()> {
    let mut size = 4 + 1 + 16 + 4;
    size += 4 + manifest.blobs.len() * (16 + 8 + 4);
    size += 4 + manifest.memo.len() * 32;
    size += 4;
    for (name, stage) in &manifest.stages {
        size += 4 + name.len() + 4 + 4;
        size += stage
            .deps
            .iter()
            .map(|(dep, _)| 4 + dep.len() + 16)
            .sum::<usize>();
        size += stage
            .units
            .values()
            .map(|unit| 4 + 16 + 4 + unit.lookups.len() + 4 + unit.payload.len())
            .sum::<usize>();
    }
    let mut buf = Packet::new(size);
    buf.p4(MAGIC);
    buf.p1(FORMAT);
    put_key(&mut buf, fingerprint());
    buf.p4(manifest.generation as i32);
    buf.p4(manifest.blobs.len() as i32);
    for (key, loc) in &manifest.blobs {
        put_key(&mut buf, *key);
        buf.p8(loc.offset as i64);
        buf.p4(loc.len as i32);
    }
    buf.p4(manifest.memo.len() as i32);
    for (key, content) in &manifest.memo {
        put_key(&mut buf, *key);
        put_key(&mut buf, *content);
    }
    buf.p4(manifest.stages.len() as i32);
    for (name, stage) in &manifest.stages {
        put_bytes(&mut buf, name.as_bytes());
        buf.p4(stage.deps.len() as i32);
        for (dep, key) in &stage.deps {
            put_bytes(&mut buf, dep.as_bytes());
            put_key(&mut buf, *key);
        }
        buf.p4(stage.units.len() as i32);
        for (id, unit) in &stage.units {
            buf.p4(*id as i32);
            put_key(&mut buf, unit.sources);
            put_bytes(&mut buf, &unit.lookups);
            put_bytes(&mut buf, &unit.payload);
        }
    }
    debug_assert_eq!(buf.pos, size);
    let temp = dir.join("manifest.tmp");
    let mut file = File::create(&temp)?;
    file.write_all(&buf.data)?;
    file.sync_all()?;
    drop(file);
    fs::rename(&temp, dir.join("manifest"))?;
    Ok(())
}

#[cfg(unix)]
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<()> {
    file.read_exact_at(buf, offset)
}

#[cfg(windows)]
fn read_at(file: &File, mut buf: &mut [u8], mut offset: u64) -> io::Result<()> {
    while !buf.is_empty() {
        match file.seek_read(buf, offset)? {
            0 => return Err(io::ErrorKind::UnexpectedEof.into()),
            read => {
                buf = &mut buf[read..];
                offset += read as u64;
            }
        }
    }
    Ok(())
}
