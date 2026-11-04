//! OpenRS2 archive integration (https://archive.openrs2.org).
//!
//! The rev-647 reference cache (`expected/`) is not kept in git: a fresh clone
//! fetches it once with `cargo fetch-expected`, and the CRC gate reads the
//! reference tables straight from the archive's API.
//!
//! Endpoints used (scope `runescape`):
//!   `/caches/runescape/<id>/disk.zip`                         the whole dat2 cache, under `cache/`
//!   `/caches/runescape/<id>/keys.json`                        validated XTEA keys
//!   `/caches/runescape/<id>/archives/<a>/groups/<g>.dat`      one raw group container
//!
//! A group fetched from the API carries no 2-byte version trailer, which is
//! exactly the form the client hashes for its login CRCs — so the CRC of
//! `(255, a)` from the API is the archive CRC `archive_crc::EXPECTED` pins.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

pub const BASE_URL: &str = "https://archive.openrs2.org";
pub const SCOPE: &str = "runescape";

/// OpenRS2 cache id of the reference cache for the selected revision.
pub const fn default_cache_id() -> Option<u32> {
    if cfg!(rev = "647") { Some(786) } else { None }
}

pub fn disk_zip_url(id: u32) -> String {
    format!("{BASE_URL}/caches/{SCOPE}/{id}/disk.zip")
}

pub fn keys_url(id: u32) -> String {
    format!("{BASE_URL}/caches/{SCOPE}/{id}/keys.json")
}

pub fn group_url(id: u32, archive: u8, group: u32) -> String {
    format!("{BASE_URL}/caches/{SCOPE}/{id}/archives/{archive}/groups/{group}.dat")
}

/// GET `url` into memory. The body reader is unbounded, unlike
/// `read_to_vec`, so a large reference table is never truncated.
pub fn get_bytes(url: &str) -> Result<Vec<u8>> {
    let mut response = ureq::get(url)
        .call()
        .with_context(|| format!("GET {url}"))?;
    let mut buf = Vec::new();
    response
        .body_mut()
        .as_reader()
        .read_to_end(&mut buf)
        .with_context(|| format!("reading body of {url}"))?;
    Ok(buf)
}

/// GET `url` and stream it to `dest`. The body lands in `dest.part` first and
/// is renamed only once complete, so an interrupted download is never mistaken
/// for a finished one.
pub fn download_to_file(url: &str, dest: &Path) -> Result<u64> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let part = dest.with_extension("part");
    let mut response = ureq::get(url)
        .call()
        .with_context(|| format!("GET {url}"))?;
    let mut out = File::create(&part).with_context(|| format!("creating {}", part.display()))?;
    let bytes = io::copy(&mut response.body_mut().as_reader(), &mut out)
        .with_context(|| format!("downloading {url}"))?;
    drop(out);
    fs::rename(&part, dest)?;
    Ok(bytes)
}

/// Fetch group `(archive, group)` of cache `id`, memoised on disk under
/// `cache_dir/<id>/<archive>/<group>.dat` so repeat runs stay offline.
pub fn fetch_group_cached(cache_dir: &Path, id: u32, archive: u8, group: u32) -> Result<Vec<u8>> {
    let path = cache_dir
        .join(id.to_string())
        .join(archive.to_string())
        .join(format!("{group}.dat"));
    if let Ok(bytes) = fs::read(&path) {
        return Ok(bytes);
    }
    let bytes = get_bytes(&group_url(id, archive, group))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, &bytes)?;
    Ok(bytes)
}

/// The bare file name of a disk.zip entry, if it is a dat2 cache file.
///
/// Only `main_file_cache.dat2` and `main_file_cache.idx<N>` are accepted, and
/// only by their last path component — an entry can never place a file outside
/// the output directory, whatever its path says.
pub fn cache_file_name(entry_path: &str) -> Option<String> {
    let name = entry_path.rsplit(['/', '\\']).next()?;
    let is_index = name
        .strip_prefix("main_file_cache.idx")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    (name == "main_file_cache.dat2" || is_index).then(|| name.to_string())
}

/// Extract the cache files of an OpenRS2 `disk.zip` flat into `out_dir`.
/// Returns the file names written, sorted.
pub fn extract_disk_zip(zip_path: &Path, out_dir: &Path) -> Result<Vec<String>> {
    let file = File::open(zip_path).with_context(|| format!("opening {}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("reading {} as a zip", zip_path.display()))?;
    fs::create_dir_all(out_dir)?;
    let mut written = Vec::new();
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = cache_file_name(&entry.name()?) else {
            continue;
        };
        let mut out = File::create(out_dir.join(&name))?;
        io::copy(&mut entry, &mut out)?;
        written.push(name);
    }
    if !written.iter().any(|n| n == "main_file_cache.dat2") {
        bail!(
            "{} holds no main_file_cache.dat2 — not a dat2 disk cache",
            zip_path.display()
        );
    }
    written.sort();
    Ok(written)
}

/// One XTEA key as OpenRS2 serves it. Extra fields (`name`, `mapsquare`,
/// `name_hash`) are ignored, so this reads both the API's `keys.json` and the
/// committed `content/<rev>/keys.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct XteaKey {
    pub archive: u32,
    pub group: u32,
    pub key: [i32; 4],
}

pub fn parse_keys(json: &str) -> Result<Vec<XteaKey>> {
    Ok(serde_json::from_str(json)?)
}

/// How two key sets disagree, by `(archive, group)`.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct KeyDiff {
    pub only_local: Vec<(u32, u32)>,
    pub only_remote: Vec<(u32, u32)>,
    pub different: Vec<(u32, u32)>,
}

impl KeyDiff {
    pub fn is_empty(&self) -> bool {
        self.only_local.is_empty() && self.only_remote.is_empty() && self.different.is_empty()
    }
}

pub fn diff_keys(local: &[XteaKey], remote: &[XteaKey]) -> KeyDiff {
    let index = |keys: &[XteaKey]| -> BTreeMap<(u32, u32), [i32; 4]> {
        keys.iter().map(|k| ((k.archive, k.group), k.key)).collect()
    };
    let (local, remote) = (index(local), index(remote));
    let mut diff = KeyDiff::default();
    for (id, key) in &local {
        match remote.get(id) {
            None => diff.only_local.push(*id),
            Some(other) if other != key => diff.different.push(*id),
            Some(_) => {}
        }
    }
    diff.only_remote = remote
        .keys()
        .filter(|id| !local.contains_key(id))
        .copied()
        .collect();
    diff
}

/// One archive whose reference table differs between a local cache and OpenRS2.
#[derive(Debug, PartialEq, Eq)]
pub struct ArchiveCrcMismatch {
    pub archive: u8,
    pub local: Option<u32>,
    pub remote: u32,
}

/// Compare the reference-table CRC of each archive in `archives` between the
/// dat2 cache in `cache` and OpenRS2 cache `id`. Empty when they agree.
pub fn verify_reference_tables(
    cache: &Path,
    id: u32,
    archives: &[u8],
    api_cache_dir: &Path,
) -> Result<Vec<ArchiveCrcMismatch>> {
    let mut store = rs_dat2::CacheStore::open(cache)
        .with_context(|| format!("opening dat2 cache at {}", cache.display()))?;
    let mut mismatches = Vec::new();
    for &archive in archives {
        let remote = fetch_group_cached(api_cache_dir, id, 255, archive as u32)?;
        let remote = rs_dat2::compute_group_crc(&remote);
        let local = store
            .read_raw_group_bytes(255, archive as u32)?
            .map(|raw| rs_dat2::compute_group_crc(&raw));
        if local != Some(remote) {
            mismatches.push(ArchiveCrcMismatch {
                archive,
                local,
                remote,
            });
        }
    }
    Ok(mismatches)
}

/// Where `cargo fetch-expected` keeps the downloaded zip and API responses.
pub fn default_work_dir() -> PathBuf {
    PathBuf::from("target").join("openrs2")
}

/// Decompress and parse a raw `(255, a)` reference-table container.
pub fn parse_reference_container(raw: &[u8]) -> Result<rs_dat2::reference::ReferenceTable> {
    let data = rs_dat2::compression::decompress(raw).context("decompressing reference table")?;
    rs_dat2::reference::parse_reference_table(&data).context("parsing reference table")
}

/// How two reference tables of the same archive disagree, group by group.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TableDiff {
    /// Groups present only in ours.
    pub only_ours: Vec<u32>,
    /// Groups present only in OpenRS2's.
    pub only_theirs: Vec<u32>,
    /// Groups in both whose CRC differs.
    pub crc: Vec<u32>,
    /// Groups in both whose CRC agrees but whose version does not.
    pub version: Vec<u32>,
    /// The table header (format, table version, named, whirlpooled) differs.
    pub header: bool,
}

impl TableDiff {
    /// No group differs. The container CRC can still differ — the table may
    /// be encoded or compressed differently around identical groups.
    pub fn groups_agree(&self) -> bool {
        self.only_ours.is_empty()
            && self.only_theirs.is_empty()
            && self.crc.is_empty()
            && self.version.is_empty()
    }

    /// Every group that differs in any way, sorted.
    pub fn groups(&self) -> Vec<u32> {
        let mut all: Vec<u32> = [&self.only_ours, &self.only_theirs, &self.crc, &self.version]
            .into_iter()
            .flatten()
            .copied()
            .collect();
        all.sort_unstable();
        all
    }
}

pub fn diff_reference_tables(
    ours: &rs_dat2::reference::ReferenceTable,
    theirs: &rs_dat2::reference::ReferenceTable,
) -> TableDiff {
    let index = |t: &rs_dat2::reference::ReferenceTable| -> BTreeMap<u32, (u32, u32)> {
        t.groups
            .iter()
            .map(|g| (g.id, (g.crc, g.version)))
            .collect()
    };
    let (o, t) = (index(ours), index(theirs));
    let mut diff = TableDiff {
        header: (ours.format, ours.version, ours.named, ours.whirlpooled)
            != (
                theirs.format,
                theirs.version,
                theirs.named,
                theirs.whirlpooled,
            ),
        ..TableDiff::default()
    };
    for (id, (crc, version)) in &o {
        match t.get(id) {
            None => diff.only_ours.push(*id),
            Some((c, _)) if c != crc => diff.crc.push(*id),
            Some((_, v)) if v != version => diff.version.push(*id),
            Some(_) => {}
        }
    }
    diff.only_theirs = t.keys().filter(|id| !o.contains_key(id)).copied().collect();
    diff
}
