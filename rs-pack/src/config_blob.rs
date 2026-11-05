//! Config blob — single-file container for all config .dat/.idx pairs.
//!
//! Matches 2004scape's `data/pack/client/config` Jagfile concept: one binary
//! that holds every config type's PackedData output. Build-cache reads this
//! blob and assembles JS5 archive 2 from it.
//!
//! ## Format
//!
//! ```text
//! magic:      b"JCFG" (4 bytes)
//! file_count: u16 BE
//! For each file:
//!   name_len:  u16 BE
//!   name:      [u8; name_len]  (e.g. "flu.dat", "flu.idx")
//!   data_len:  u32 BE
//!   data:      [u8; data_len]
//! ```

use crate::pack::util::verify_crc;
use crate::{CONFIG_TYPE_CRCS, config_crc};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

const MAGIC: &[u8; 4] = b"JCFG";

/// A config blob bundling multiple named .dat/.idx files.
pub struct ConfigBlob {
    files: BTreeMap<String, Vec<u8>>,
}

impl ConfigBlob {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, name: &str, data: Vec<u8>) {
        self.files.insert(name.to_string(), data);
    }

    pub fn add_packed_data(&mut self, type_name: &str, dat: Vec<u8>, idx: Vec<u8>) {
        self.files.insert(format!("{type_name}.dat"), dat);
        self.files.insert(format!("{type_name}.idx"), idx);
    }

    pub fn verify(&self) -> Result<()> {
        for &(name, expected) in CONFIG_TYPE_CRCS.iter().chain(config_crc::GENERIC) {
            let data = self
                .get(&format!("{name}.dat"))
                .with_context(|| format!("Missing client config: {name}"))?;
            verify_crc(name, data, expected);
        }
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let mut buf = Vec::new();
        buf.extend_from_slice(MAGIC);

        let count = self.files.len() as u16;
        buf.push((count >> 8) as u8);
        buf.push(count as u8);

        for (name, data) in &self.files {
            let name_bytes = name.as_bytes();
            let name_len = name_bytes.len() as u16;
            buf.push((name_len >> 8) as u8);
            buf.push(name_len as u8);
            buf.extend_from_slice(name_bytes);

            let data_len = data.len() as u32;
            buf.push((data_len >> 24) as u8);
            buf.push((data_len >> 16) as u8);
            buf.push((data_len >> 8) as u8);
            buf.push(data_len as u8);
            buf.extend_from_slice(data);
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &buf)
            .with_context(|| format!("writing config blob to {}", path.display()))?;

        tracing::info!(
            "  config blob: {} files, {} bytes → {}",
            self.files.len(),
            buf.len(),
            path.display()
        );
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)
            .with_context(|| format!("reading config blob from {}", path.display()))?;

        anyhow::ensure!(data.len() >= 6, "config blob too short");
        anyhow::ensure!(&data[0..4] == MAGIC, "bad config blob magic");

        let file_count = u16::from_be_bytes([data[4], data[5]]) as usize;
        let mut pos = 6;
        let mut files = BTreeMap::new();

        for _ in 0..file_count {
            anyhow::ensure!(pos + 2 <= data.len(), "truncated name length");
            let name_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
            pos += 2;

            anyhow::ensure!(pos + name_len <= data.len(), "truncated name");
            let name = std::str::from_utf8(&data[pos..pos + name_len])
                .context("invalid name in config blob")?
                .to_string();
            pos += name_len;

            anyhow::ensure!(pos + 4 <= data.len(), "truncated data length");
            let data_len =
                u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]])
                    as usize;
            pos += 4;

            anyhow::ensure!(pos + data_len <= data.len(), "truncated data");
            files.insert(name, data[pos..pos + data_len].to_vec());
            pos += data_len;
        }

        Ok(Self { files })
    }

    /// Read a PackedData .dat/.idx pair from the blob and return entries.
    ///
    /// This replaces `assemble::read_packed_data` — same parsing, but from
    /// the blob's in-memory bytes instead of individual files on disk.
    pub fn read_packed_data(&self, type_name: &str) -> Result<BTreeMap<u32, Vec<u8>>> {
        let dat = self.files.get(&format!("{type_name}.dat"));
        let idx = self.files.get(&format!("{type_name}.idx"));

        let (Some(dat), Some(idx)) = (dat, idx) else {
            return Ok(BTreeMap::new());
        };

        anyhow::ensure!(dat.len() >= 2, "{type_name}.dat too short");
        anyhow::ensure!(idx.len() >= 2, "{type_name}.idx too short");

        let dat_count = u16::from_be_bytes([dat[0], dat[1]]) as u32;
        let idx_count = u16::from_be_bytes([idx[0], idx[1]]) as u32;
        anyhow::ensure!(
            dat_count == idx_count,
            "{type_name}: dat count ({dat_count}) != idx count ({idx_count})"
        );

        let count = dat_count;
        anyhow::ensure!(
            idx.len() >= 2 + (count as usize) * 2,
            "{type_name}.idx too short for {count} entries"
        );

        let mut entries = BTreeMap::new();
        let mut dat_offset = 2usize;

        for id in 0..count {
            let idx_off = 2 + (id as usize) * 2;
            let entry_len = u16::from_be_bytes([idx[idx_off], idx[idx_off + 1]]) as usize;

            if entry_len > 0 {
                anyhow::ensure!(
                    dat_offset + entry_len <= dat.len(),
                    "{type_name}: dat underflow at entry {id}"
                );
                entries.insert(id, dat[dat_offset..dat_offset + entry_len].to_vec());
            }

            dat_offset += entry_len;
        }

        Ok(entries)
    }

    pub fn has(&self, type_name: &str) -> bool {
        self.files.contains_key(&format!("{type_name}.dat"))
    }

    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.files.get(name).map(|v| v.as_slice())
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(|s| s.as_str())
    }
}
