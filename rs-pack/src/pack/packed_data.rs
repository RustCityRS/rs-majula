use crate::pack::pack_registry::PackId;
use anyhow::{Result, ensure};
#[cfg(since_647)]
use rs_io::cp1252;
use rs_ver::STRING_TERMINATOR;
use std::path::Path;
use tracing::debug;

#[derive(Default)]
pub struct PackedData {
    pub dat: Vec<u8>,
    pub idx: Vec<u8>,
    count: PackId,
    entry_start: usize,
}

impl PackedData {
    pub fn entries<'a>(
        dat: &'a [u8],
        idx: &'a [u8],
    ) -> Result<impl ExactSizeIterator<Item = (u16, &'a [u8])>> {
        ensure!(dat.len() >= 2 && idx.len() >= 2, "Missing config count");
        let count = u16::from_be_bytes([dat[0], dat[1]]) as usize;
        ensure!(dat[..2] == idx[..2], "Config data and index counts differ");
        ensure!(idx.len() >= 2 + (count << 1), "Truncated config index");
        let lengths = idx[2..2 + (count << 1)].as_chunks::<2>().0.iter();
        let length = |bytes: &[u8; 2]| u16::from_be_bytes(*bytes) as usize;
        ensure!(
            2 + lengths.clone().map(length).sum::<usize>() <= dat.len(),
            "Truncated config data"
        );
        let mut offset = 2;
        Ok(lengths.enumerate().map(move |(id, bytes)| {
            let start = offset;
            offset += length(bytes);
            (id as u16, &dat[start..offset])
        }))
    }

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

    #[cfg(since_647)]
    pub fn pvarint(&mut self, value: u32) {
        let mut shift = 28;
        while shift > 0 && value >> shift == 0 {
            shift -= 7;
        }
        while shift > 0 {
            self.p1((value >> shift) as u8 | 0x80);
            shift -= 7;
        }
        self.p1(value as u8 & 0x7f);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_entries_preserve_gaps_and_check_lengths() {
        let dat = [0, 3, 0, 1, 7, 0];
        let idx = [0, 3, 0, 1, 0, 0, 0, 3];
        assert_eq!(
            PackedData::entries(&dat, &idx).unwrap().collect::<Vec<_>>(),
            vec![(0, &dat[2..3]), (1, &dat[3..3]), (2, &dat[3..6])]
        );
        assert!(PackedData::entries(&[], &idx).is_err());
        assert!(PackedData::entries(&dat, &idx[..6]).is_err());
        assert!(PackedData::entries(&dat[..5], &idx).is_err());
        assert!(PackedData::entries(&dat, &[0, 0]).is_err());
    }
}
