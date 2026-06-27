use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::Path;

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
        out.push_str("# name            idx0  cache          constant       status\n");
        for a in &self.archives {
            let (status, mismatch) = crc_status(a.computed, a.expected);
            mismatches += mismatch as usize;
            let idx0 = a
                .idx0_file
                .map_or_else(|| "-".to_string(), |n| n.to_string());
            let expected = a
                .expected
                .map_or_else(|| "-".to_string(), |e| e.to_string());
            writeln!(
                out,
                "{:<16}{:<6}{:>11}  {:>13}  {status}",
                a.name, idx0, a.computed, expected
            )
            .unwrap();
        }

        out.push_str("\n# Per-config-type client .dat CRCs (constant = config_crc::*)\n");
        out.push_str("# type            cache          constant       status\n");
        for c in &self.configs {
            let (status, mismatch) = crc_status(c.computed, Some(c.expected));
            mismatches += mismatch as usize;
            writeln!(
                out,
                "{:<16}{:>11}  {:>13}  {status}",
                c.name, c.computed, c.expected
            )
            .unwrap();
        }

        #[cfg(since_244)]
        if !self.js5_ondemand.is_empty() {
            out.push_str("\n# JS5 on-demand CRCs (from the version list)\n");
            out.push_str(
                "# listed = CRC the version list records; recomputed = getcrc over the stored blob minus its 2-byte version trailer\n",
            );
            out.push_str("# table   id      version  listed         recomputed     status\n");
            for r in &self.js5_ondemand {
                let (status, mismatch, recomputed) = match r.recomputed {
                    Some(rc) => {
                        let (s, m) = crc_status(rc, Some(r.expected));
                        (s, m, rc.to_string())
                    }
                    None => ("absent", false, "-".to_string()),
                };
                mismatches += mismatch as usize;
                writeln!(
                    out,
                    "{:<8}{:<8}{:<9}{:>11}  {:>13}  {status}",
                    r.table, r.id, r.version, r.expected, recomputed
                )
                .unwrap();
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
    hash: i32,
    packed: usize,
    unpacked: usize,
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
            self.unknown_jag.push(UnknownJagFile {
                archive: archive.to_string(),
                hash,
                packed,
                unpacked,
            });
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
            out.push_str("# archive         hash          packed   unpacked\n");
            for u in &self.unknown_jag {
                writeln!(
                    out,
                    "{:<16}{:>11}  {:>8}  {:>8}",
                    u.archive, u.hash, u.packed, u.unpacked
                )
                .unwrap();
            }
        }

        if !self.dat_trailing.is_empty() {
            out.push_str("\n## Trailing bytes past the last entry in a config .dat\n");
            out.push_str("# type            bytes\n");
            for (name, bytes) in &self.dat_trailing {
                writeln!(out, "{name:<16}{bytes}").unwrap();
            }
        }

        if !self.record_trailing.is_empty() {
            out.push_str("\n## Trailing bytes inside a config record (after its 0 terminator)\n");
            out.push_str("# type            id      bytes\n");
            for r in &self.record_trailing {
                writeln!(out, "{:<16}{:<8}{}", r.config_type, r.id, r.bytes).unwrap();
            }
        }

        #[cfg(since_244)]
        if !self.js5_unread.is_empty() {
            out.push_str("\n## Unread JS5 blobs (present in the cache, never extracted)\n");
            out.push_str("# index  file    size       reason\n");
            for u in &self.js5_unread {
                writeln!(out, "{:<7}{:<8}{:<11}{}", u.index, u.file, u.size, u.reason).unwrap();
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
