use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fmt::{Display, Write as _};
use std::path::{Path, PathBuf};

use rs_io::crc;
use rs_io::jag::JagFile;
use tracing::{info, warn};

pub struct RecordLeftover {
    pub config_type: &'static str,
    pub id: u16,
    pub bytes: usize,
}

struct ArchiveCrc {
    name: String,
    idx0_file: Option<usize>,
    computed: i32,
    expected: Option<i32>,
}

struct ConfigCrc {
    name: String,
    computed: i32,
    expected: i32,
}

#[cfg(since_244)]
struct Js5OndemandCrc {
    table: &'static str,
    id: usize,
    version: u16,
    expected: i32,
    recomputed: Option<i32>,
}

#[derive(Default)]
pub struct CrcReport {
    archives: Vec<ArchiveCrc>,
    configs: Vec<ConfigCrc>,
    #[cfg(since_244)]
    js5_ondemand: Vec<Js5OndemandCrc>,
}

impl CrcReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn archive(&mut self, name: &str, raw: &[u8], expected: Option<i32>) {
        self.archives.push(ArchiveCrc {
            name: name.to_string(),
            idx0_file: idx0_of(name),
            computed: crc::getcrc(raw, 0, raw.len()),
            expected,
        });
    }

    pub fn config(&mut self, name: &str, client_dat: &[u8], expected: i32) {
        self.configs.push(ConfigCrc {
            name: name.to_string(),
            computed: crc::getcrc(client_dat, 0, client_dat.len()),
            expected,
        });
    }

    #[cfg(since_244)]
    pub fn js5_ondemand(
        &mut self,
        table: &'static str,
        id: usize,
        version: u16,
        expected: i32,
        recomputed: Option<i32>,
    ) {
        self.js5_ondemand.push(Js5OndemandCrc {
            table,
            id,
            version,
            expected,
            recomputed,
        });
    }

    pub fn write(&self, output_dir: &Path) -> std::io::Result<()> {
        let dir = output_dir.join("_crc");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("report.txt");

        let mut out = String::new();
        let mut mismatches = 0usize;

        out.push_str(
            "# cache    = CRC computed from the cache being unpacked (the source/expected dir)\n",
        );
        out.push_str(
            "# constant = hard-coded jag_crc::* / config_crc::* value for the active revision\n",
        );
        out.push('\n');
        out.push_str("# JAG archive CRCs (cache CRC over each archive's raw bytes)\n");
        out.push_str("# idx0 = JS5 index-0 file slot the archive lives in (since 244)\n");
        out.push_str(&archive_crc_row(
            "# name", "idx0", "cache", "constant", "status",
        ));
        for a in &self.archives {
            let (status, mismatch) = crc_status(a.computed, a.expected);
            mismatches += mismatch as usize;
            let idx0 = a
                .idx0_file
                .map_or_else(|| "-".to_string(), |n| n.to_string());
            let expected = a
                .expected
                .map_or_else(|| "-".to_string(), |e| e.to_string());
            out.push_str(&archive_crc_row(
                &a.name, idx0, a.computed, expected, status,
            ));
        }

        out.push_str("\n# Per-config-type client .dat CRCs (constant = config_crc::*)\n");
        out.push_str(&config_crc_row("# type", "cache", "constant", "status"));
        for c in &self.configs {
            let (status, mismatch) = crc_status(c.computed, Some(c.expected));
            mismatches += mismatch as usize;
            out.push_str(&config_crc_row(&c.name, c.computed, c.expected, status));
        }

        #[cfg(since_244)]
        if !self.js5_ondemand.is_empty() {
            out.push_str("\n# JS5 on-demand CRCs (from the version list)\n");
            out.push_str(
                "# listed = CRC the version list records; recomputed = getcrc over the stored blob minus its 2-byte version trailer\n",
            );
            out.push_str(&js5_ondemand_row(
                "# table",
                "id",
                "version",
                "listed",
                "recomputed",
                "status",
            ));
            for r in &self.js5_ondemand {
                let (status, mismatch, recomputed) = match r.recomputed {
                    Some(rc) => {
                        let (s, m) = crc_status(rc, Some(r.expected));
                        (s, m, rc.to_string())
                    }
                    None => ("absent", false, "-".to_string()),
                };
                mismatches += mismatch as usize;
                out.push_str(&js5_ondemand_row(
                    r.table, r.id, r.version, r.expected, recomputed, status,
                ));
            }
        }

        std::fs::write(&path, &out)?;
        if mismatches == 0 {
            info!("CRC report: all CRCs match - {}", path.display());
        } else {
            warn!(
                "CRC report: {mismatches} mismatch(es) - see {}",
                path.display()
            );
        }
        Ok(())
    }
}

struct UnknownJagFile {
    archive: String,
    id: usize,
    hash: i32,
    packed: usize,
    unpacked: usize,
    crc: i32,
    candidates: Vec<String>,
}

#[cfg(since_244)]
struct Js5Unread {
    index: usize,
    file: usize,
    size: usize,
    reason: &'static str,
}

#[derive(Default)]
pub struct LeftoverReport {
    unknown_jag: Vec<UnknownJagFile>,
    record_trailing: Vec<RecordLeftover>,
    dat_trailing: Vec<(String, usize)>,
    #[cfg(since_244)]
    js5_unread: Vec<Js5Unread>,
    #[cfg(since_244)]
    extra_indices: Vec<usize>,
}

impl LeftoverReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn scan_jag(&mut self, archive: &str, jag: &JagFile, known: &[i32]) {
        let known: HashSet<i32> = known.iter().copied().collect();
        for i in 0..jag.file_count {
            let hash = jag.file_hash(i);
            if known.contains(&hash) {
                continue;
            }
            let packed = jag.file_packs.as_ref().map_or(0, |p| p[i].max(0) as usize);
            let unpacked = jag
                .file_unpacks
                .as_ref()
                .map_or(0, |u| u[i].max(0) as usize);
            let crc = jag
                .get(i)
                .map_or(0, |p| crc::getcrc(&p.data, 0, p.data.len()));
            self.unknown_jag.push(UnknownJagFile {
                archive: archive.to_string(),
                id: i,
                hash,
                packed,
                unpacked,
                crc,
                candidates: Vec::new(),
            });
        }
    }

    pub fn crack_unknown_names(&mut self) {
        if self.unknown_jag.is_empty() {
            return;
        }
        let targets: Vec<i32> = self.unknown_jag.iter().map(|u| u.hash).collect();
        let cracked = super::namecrack::crack(&targets);
        for entry in &mut self.unknown_jag {
            if let Some(names) = cracked.get(&entry.hash) {
                entry.candidates = names.clone();
            }
        }
    }

    pub fn add_config_leftovers(
        &mut self,
        records: Vec<RecordLeftover>,
        dat_trailing: Vec<(String, usize)>,
    ) {
        self.record_trailing.extend(records);
        self.dat_trailing.extend(dat_trailing);
    }

    #[cfg(since_244)]
    pub fn js5_unread(&mut self, index: usize, file: usize, size: usize, reason: &'static str) {
        self.js5_unread.push(Js5Unread {
            index,
            file,
            size,
            reason,
        });
    }

    #[cfg(since_244)]
    pub fn extra_index(&mut self, index: usize) {
        self.extra_indices.push(index);
    }

    fn is_empty(&self) -> bool {
        let empty = self.unknown_jag.is_empty()
            && self.record_trailing.is_empty()
            && self.dat_trailing.is_empty();
        #[cfg(since_244)]
        let empty = empty && self.js5_unread.is_empty() && self.extra_indices.is_empty();
        empty
    }

    pub fn write(&self, output_dir: &Path) -> std::io::Result<()> {
        let dir = output_dir.join("_leftover");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("report.txt");

        if self.is_empty() {
            std::fs::write(&path, "No unaccounted data detected during unpack.\n")?;
            info!("Leftover report: no unaccounted data - {}", path.display());
            return Ok(());
        }

        let mut out = String::new();
        out.push_str(
            "# Data present in the source cache that the unpacker did NOT consume.\n\
             # Each entry usually marks a cache delta a newer revision introduced\n\
             # that the engine/unpacker does not handle yet.\n",
        );

        if !self.unknown_jag.is_empty() {
            out.push_str("\n## Unknown files inside JAG archives (hash has no known name)\n");
            out.push_str(&unknown_row(
                "# archive",
                "id",
                "hash",
                "packed",
                "unpacked",
                "crc",
            ));
            for u in &self.unknown_jag {
                out.push_str(&unknown_row(
                    &u.archive, u.id, u.hash, u.packed, u.unpacked, u.crc,
                ));
                if u.candidates.is_empty() {
                    out.push_str("    candidates: none within search bounds\n");
                } else {
                    writeln!(out, "    candidates: {}", u.candidates.join(", ")).unwrap();
                }
            }
        }

        if !self.dat_trailing.is_empty() {
            out.push_str("\n## Trailing bytes past the last entry in a config .dat\n");
            out.push_str(&dat_trailing_row("# type", "bytes"));
            for (name, bytes) in &self.dat_trailing {
                out.push_str(&dat_trailing_row(name, bytes));
            }
        }

        if !self.record_trailing.is_empty() {
            out.push_str("\n## Trailing bytes inside a config record (after its 0 terminator)\n");
            out.push_str(&record_trailing_row("# type", "id", "bytes"));
            for r in &self.record_trailing {
                out.push_str(&record_trailing_row(r.config_type, r.id, r.bytes));
            }
        }

        #[cfg(since_244)]
        if !self.js5_unread.is_empty() {
            out.push_str("\n## Unread JS5 blobs (present in the cache, never extracted)\n");
            out.push_str(&js5_unread_row("# index", "file", "size", "reason"));
            for u in &self.js5_unread {
                out.push_str(&js5_unread_row(u.index, u.file, u.size, u.reason));
            }
        }

        #[cfg(since_244)]
        if !self.extra_indices.is_empty() {
            out.push_str("\n## JS5 indices present on disk but not handled by the unpacker\n");
            for n in &self.extra_indices {
                writeln!(out, "main_file_cache.idx{n}").unwrap();
            }
        }

        std::fs::write(&path, &out)?;

        // One concise warning per category - the file holds the detail.
        warn!(
            "Leftover report: unaccounted data found - see {}",
            path.display()
        );
        if !self.unknown_jag.is_empty() {
            warn!(
                "  {} unknown file(s) across JAG archives",
                self.unknown_jag.len()
            );
        }
        if !self.record_trailing.is_empty() {
            let bytes: usize = self.record_trailing.iter().map(|r| r.bytes).sum();
            warn!(
                "  {} config record(s) with {bytes} trailing byte(s)",
                self.record_trailing.len()
            );
        }
        if !self.dat_trailing.is_empty() {
            warn!(
                "  {} config .dat(s) with trailing bytes",
                self.dat_trailing.len()
            );
        }
        #[cfg(since_244)]
        if !self.js5_unread.is_empty() {
            warn!("  {} unread JS5 blob(s)", self.js5_unread.len());
        }
        #[cfg(since_244)]
        if !self.extra_indices.is_empty() {
            warn!("  {} unhandled JS5 index file(s)", self.extra_indices.len());
        }

        Ok(())
    }
}

pub struct PackDiffReport {
    committed_dir: PathBuf,
    unpacked_dir: PathBuf,
    files: Vec<PackFileDiff>,
    unchanged_files: usize,
    not_produced: Vec<String>,
    maps: Option<MapDiff>,
}

struct PackFileDiff {
    name: String,
    is_new_file: bool,
    added: Vec<IdEntry>,
    removed: Vec<IdEntry>,
}

struct IdEntry {
    id: String,
    name: String,
}

struct MapDiff {
    unpacked_dir: PathBuf,
    added: Vec<(u8, u8)>,
    removed: Vec<(u8, u8)>,
    unchanged: usize,
}

impl MapDiff {
    fn has_delta(&self) -> bool {
        !self.added.is_empty() || !self.removed.is_empty()
    }

    fn section(&self) -> String {
        let mut out = format!(
            "\n## maps (m<x>_<z>.jm2 squares): +{} added, -{} removed\n",
            self.added.len(),
            self.removed.len(),
        );
        for (x, z) in &self.added {
            writeln!(out, "  + m{x}_{z}").unwrap();
        }
        for (x, z) in &self.removed {
            writeln!(out, "  - m{x}_{z}").unwrap();
        }
        out
    }
}

impl PackDiffReport {
    pub fn compare(committed_dir: &Path, unpacked_dir: &Path) -> std::io::Result<Self> {
        let committed = collect_pack_files(committed_dir)?;
        let unpacked = collect_pack_files(unpacked_dir)?;

        let mut report = Self {
            committed_dir: committed_dir.to_path_buf(),
            unpacked_dir: unpacked_dir.to_path_buf(),
            files: Vec::new(),
            unchanged_files: 0,
            not_produced: committed
                .keys()
                .filter(|name| !unpacked.contains_key(*name))
                .cloned()
                .collect(),
            maps: None,
        };

        for (name, u_path) in &unpacked {
            let u_map = parse_pack(u_path)?;
            let (c_map, is_new_file) = match committed.get(name) {
                Some(p) => (parse_pack(p)?, false),
                None => (BTreeMap::new(), true),
            };

            let mut added = Vec::new();
            for (id, u_name) in &u_map {
                if !c_map.contains_key(id) {
                    added.push(IdEntry {
                        id: id.clone(),
                        name: u_name.clone(),
                    });
                }
            }
            let mut removed = Vec::new();
            for (id, c_name) in &c_map {
                if !u_map.contains_key(id) {
                    removed.push(IdEntry {
                        id: id.clone(),
                        name: c_name.clone(),
                    });
                }
            }

            if added.is_empty() && removed.is_empty() {
                report.unchanged_files += 1;
                continue;
            }
            added.sort_by_key(|e| id_key(&e.id));
            removed.sort_by_key(|e| id_key(&e.id));
            report.files.push(PackFileDiff {
                name: name.clone(),
                is_new_file,
                added,
                removed,
            });
        }
        Ok(report)
    }

    pub fn compare_maps(
        &mut self,
        committed_maps: &Path,
        unpacked_maps: &Path,
    ) -> std::io::Result<()> {
        let committed = collect_map_squares(committed_maps)?;
        let unpacked = collect_map_squares(unpacked_maps)?;
        self.maps = Some(MapDiff {
            unpacked_dir: unpacked_maps.to_path_buf(),
            added: unpacked.difference(&committed).copied().collect(),
            removed: committed.difference(&unpacked).copied().collect(),
            unchanged: committed.intersection(&unpacked).count(),
        });
        Ok(())
    }

    pub fn write(&self, output_dir: &Path) -> std::io::Result<()> {
        let dir = output_dir.join("_packdiff");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("report.txt");

        let compared = self.files.len() + self.unchanged_files;

        let mut footer = String::new();
        if !self.not_produced.is_empty() {
            writeln!(
                footer,
                "\n# Not compared: {} committed .pack file(s) the unpacker does not produce",
                self.not_produced.len(),
            )
            .unwrap();
            footer.push_str("# (they come from authored content registries, not the cache):\n");
            writeln!(footer, "#   {}", self.not_produced.join(", ")).unwrap();
        }

        let map_delta = self.maps.as_ref().is_some_and(MapDiff::has_delta);

        if self.files.is_empty() && !map_delta {
            let mut out = format!(
                "No .pack id deltas: all {compared} unpacked .pack file(s) match {} by id.\n",
                self.committed_dir.display(),
            );
            if let Some(m) = &self.maps {
                writeln!(
                    out,
                    "No new/removed map squares: all {} square(s) match {}.",
                    m.unchanged,
                    m.unpacked_dir.display(),
                )
                .unwrap();
            }
            out.push_str(&footer);
            std::fs::write(&path, &out)?;
            info!(
                "Pack-diff report: no id deltas across {compared} unpacked .pack file(s) vs {} - {}",
                self.committed_dir.display(),
                path.display()
            );
            return Ok(());
        }

        let (mut added, mut removed) = (0usize, 0usize);
        for f in &self.files {
            added += f.added.len();
            removed += f.removed.len();
        }

        let mut out = String::new();
        out.push_str(
            "# Pack-file delta: the .pack registries THIS unpack produced vs the ones checked\n\
             # in for this revision. Entries are matched by ID (the value left of '='); a\n\
             # name-only change for an existing id is NOT a delta - only added/removed ids.\n\
             # Decoded map squares (m<x>_<z>.jm2) are diffed by presence in the maps section.\n",
        );
        writeln!(out, "# committed = {}", self.committed_dir.display()).unwrap();
        writeln!(out, "# unpacked  = {}", self.unpacked_dir.display()).unwrap();
        out.push_str(
            "#\n\
             #   + id  added   - id only in the fresh unpack (a new entry this cache introduces)\n\
             #   - id  removed - id only in this file's committed copy (no longer produced)\n",
        );
        writeln!(
            out,
            "\nSummary: {compared} unpacked .pack file(s) compared - {} with id deltas, {} identical.",
            self.files.len(),
            self.unchanged_files,
        )
        .unwrap();
        writeln!(out, "  ids: +{added} added, -{removed} removed").unwrap();
        if let Some(m) = &self.maps {
            writeln!(
                out,
                "  maps: +{} added, -{} removed, {} unchanged",
                m.added.len(),
                m.removed.len(),
                m.unchanged,
            )
            .unwrap();
        }

        for f in &self.files {
            let tag = if f.is_new_file {
                " (new pack file)"
            } else {
                ""
            };
            writeln!(
                out,
                "\n## {}{tag}: +{} added, -{} removed",
                f.name,
                f.added.len(),
                f.removed.len(),
            )
            .unwrap();
            for e in &f.added {
                writeln!(out, "  + {:<8} {}", e.id, e.name).unwrap();
            }
            for e in &f.removed {
                writeln!(out, "  - {:<8} {}", e.id, e.name).unwrap();
            }
        }

        if let Some(m) = &self.maps
            && m.has_delta()
        {
            out.push_str(&m.section());
        }

        out.push_str(&footer);
        std::fs::write(&path, &out)?;
        let map_clause = match &self.maps {
            Some(m) if m.has_delta() => {
                format!(", maps +{} -{}", m.added.len(), m.removed.len())
            }
            _ => String::new(),
        };
        warn!(
            "Pack-diff report: {} of {compared} unpacked .pack file(s) differ by id (+{added} -{removed}){map_clause} vs {} - see {}",
            self.files.len(),
            self.committed_dir.display(),
            path.display()
        );
        Ok(())
    }
}

fn collect_pack_files(root: &Path) -> std::io::Result<BTreeMap<String, PathBuf>> {
    let mut files = BTreeMap::new();
    if root.exists() {
        collect_packs_into(root, root, &mut files)?;
    }
    Ok(files)
}

fn collect_packs_into(
    root: &Path,
    dir: &Path,
    files: &mut BTreeMap<String, PathBuf>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_packs_into(root, &path, files)?;
        } else if path.extension().is_some_and(|e| e == "pack")
            && let Ok(rel) = path.strip_prefix(root)
        {
            files.insert(rel.to_string_lossy().replace('\\', "/"), path);
        }
    }
    Ok(())
}

fn parse_pack(path: &Path) -> std::io::Result<BTreeMap<String, String>> {
    let text = std::fs::read_to_string(path)?;
    let mut map = BTreeMap::new();
    for line in text.lines() {
        if let Some((id, name)) = line.split_once('=') {
            map.insert(id.trim().to_string(), name.to_string());
        }
    }
    Ok(map)
}

fn id_key(id: &str) -> (i64, String) {
    (id.parse::<i64>().unwrap_or(i64::MAX), id.to_string())
}

fn collect_map_squares(dir: &Path) -> std::io::Result<BTreeSet<(u8, u8)>> {
    let mut squares = BTreeSet::new();
    if dir.exists() {
        for entry in std::fs::read_dir(dir)? {
            if let Some(sq) = entry?
                .path()
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(parse_map_square)
            {
                squares.insert(sq);
            }
        }
    }
    Ok(squares)
}

fn parse_map_square(name: &str) -> Option<(u8, u8)> {
    let (x, z) = name
        .strip_suffix(".jm2")?
        .strip_prefix('m')?
        .split_once('_')?;
    Some((x.parse().ok()?, z.parse().ok()?))
}

fn unknown_row(
    archive: impl Display,
    id: impl Display,
    hash: impl Display,
    packed: impl Display,
    unpacked: impl Display,
    crc: impl Display,
) -> String {
    format!("{archive:<16}{id:>6}{hash:>14}{packed:>11}{unpacked:>11}{crc:>16}\n")
}

fn archive_crc_row(
    name: impl Display,
    idx0: impl Display,
    cache: impl Display,
    constant: impl Display,
    status: impl Display,
) -> String {
    format!("{name:<16}{idx0:<6}{cache:>11}{constant:>15}  {status}\n")
}

fn config_crc_row(
    ty: impl Display,
    cache: impl Display,
    constant: impl Display,
    status: impl Display,
) -> String {
    format!("{ty:<16}{cache:>11}{constant:>15}  {status}\n")
}

#[cfg(since_244)]
fn js5_ondemand_row(
    table: impl Display,
    id: impl Display,
    version: impl Display,
    listed: impl Display,
    recomputed: impl Display,
    status: impl Display,
) -> String {
    format!("{table:<8}{id:<8}{version:<9}{listed:>11}{recomputed:>15}  {status}\n")
}

fn dat_trailing_row(ty: impl Display, bytes: impl Display) -> String {
    format!("{ty:<16}{bytes}\n")
}

fn record_trailing_row(ty: impl Display, id: impl Display, bytes: impl Display) -> String {
    format!("{ty:<16}{id:<8}{bytes}\n")
}

#[cfg(since_244)]
fn js5_unread_row(
    index: impl Display,
    file: impl Display,
    size: impl Display,
    reason: impl Display,
) -> String {
    format!("{index:<8}{file:<8}{size:<11}{reason}\n")
}

fn crc_status(computed: i32, expected: Option<i32>) -> (&'static str, bool) {
    match expected {
        Some(e) if e == computed => ("OK", false),
        Some(_) => ("MISMATCH", true),
        None => ("unverified", false),
    }
}

#[cfg(since_244)]
fn idx0_of(name: &str) -> Option<usize> {
    Some(match name {
        "title" => 1,
        "config" => 2,
        "interface" => 3,
        "media" => 4,
        "versionlist" => 5,
        "textures" => 6,
        "wordenc" => 7,
        "sounds" => 8,
        _ => return None,
    })
}

#[cfg(rev = "225")]
fn idx0_of(_name: &str) -> Option<usize> {
    None
}
