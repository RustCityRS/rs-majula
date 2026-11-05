use crate::pack::pack_registry::PackId;
#[cfg(since_647)]
use rs_io::cp1252;
use rs_ver::STRING_TERMINATOR;
use std::path::Path;

use anyhow::Result;
use tracing::debug;

pub struct PackedData {
    pub dat: Vec<u8>,
    pub idx: Vec<u8>,
    count: PackId,
    entry_start: usize,
}

impl PackedData {
    pub fn new(count: PackId) -> Self {
        let mut dat = Vec::with_capacity(500_000);
        let mut idx = Vec::with_capacity((count as usize + 1) * 2);
        // Header: p2(count)
        dat.push((count >> 8) as u8);
        dat.push(count as u8);
        idx.push((count >> 8) as u8);
        idx.push(count as u8);
        let entry_start = dat.len();
        Self {
            dat,
            idx,
            count,
            entry_start,
        }
    }

    /// Start a new entry. Call this before writing opcodes for an entry.
    #[inline]
    pub const fn start_entry(&mut self) {
        self.entry_start = self.dat.len();
    }

    /// Finish the current entry. Writes the p1(0) terminator and records
    /// the entry length in the idx.
    #[inline]
    pub fn finish_entry(&mut self) {
        self.p1(0);
        self.finish_entry_raw();
    }

    #[inline]
    pub fn finish_entry_raw(&mut self) {
        let len = self.dat.len() - self.entry_start;
        self.idx.push((len >> 8) as u8);
        self.idx.push(len as u8);
    }

    #[inline]
    pub fn skip_entry(&mut self) {
        self.idx.extend_from_slice(&[0, 0]);
    }

    #[inline]
    pub fn p1(&mut self, value: u8) {
        self.dat.push(value);
    }

    #[inline]
    pub fn p2(&mut self, value: u16) {
        self.dat.push((value >> 8) as u8);
        self.p1(value as u8)
    }

    #[inline]
    pub fn p3(&mut self, value: i32) {
        self.dat.push((value >> 16) as u8);
        self.p2(value as u16);
    }

    #[inline]
    pub fn p4(&mut self, value: i32) {
        self.dat.push((value >> 24) as u8);
        self.p3(value);
    }

    #[inline]
    pub fn pbool(&mut self, value: bool) {
        self.p1(value as u8);
    }

    #[inline]
    pub fn pjstr(&mut self, value: &str) {
        #[cfg(before_647)]
        self.dat.extend_from_slice(value.as_bytes());
        #[cfg(since_647)]
        self.dat.extend_from_slice(&cp1252::encode_string(value));
        self.dat.push(STRING_TERMINATOR);
    }

    pub fn entry(&self) -> &[u8] {
        &self.dat[self.entry_start..]
    }

    /// Save the .dat and .idx files to disk.
    pub fn save(&self, base_path: &Path, name: &str) -> Result<()> {
        std::fs::create_dir_all(base_path)?;
        let dat_path = base_path.join(format!("{name}.dat"));
        let idx_path = base_path.join(format!("{name}.idx"));
        std::fs::write(&dat_path, &self.dat)?;
        std::fs::write(&idx_path, &self.idx)?;
        debug!(
            "  {name}: {count} entries, dat={dat_size} bytes, idx={idx_size} bytes",
            count = self.count,
            dat_size = self.dat.len(),
            idx_size = self.idx.len(),
        );
        Ok(())
    }
}
