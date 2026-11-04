//! PackedData — binary builder for opcode-terminated config entries.
//! Matches 2004scape's `PackedData` class in `PackShared.ts`.
//!
//! Format:
//!   dat: p2(count) [entry₀] [entry₁] ... [entryₙ₋₁]
//!   idx: p2(count) [p2(len₀)] [p2(len₁)] ... [p2(lenₙ₋₁)]
//!
//! Each entry: [p1(opcode) data...]* p1(0)

use std::path::Path;

use anyhow::Result;
use tracing::info;

pub struct PackedData {
    pub dat: Vec<u8>,
    pub idx: Vec<u8>,
    count: u32,
    entry_start: usize,
}

impl PackedData {
    pub fn new(count: u32) -> Self {
        let mut dat = Vec::with_capacity(4096);
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
    pub fn start_entry(&mut self) {
        self.entry_start = self.dat.len();
    }

    /// Finish the current entry. Writes the p1(0) terminator and records
    /// the entry length in the idx.
    pub fn finish_entry(&mut self) {
        self.p1(0); // terminator
        let len = self.dat.len() - self.entry_start;
        self.idx.push((len >> 8) as u8);
        self.idx.push(len as u8);
    }

    /// Record a zero-length (absent) entry. No data is written to dat.
    /// The idx records length=0 so the decoder can skip this entry.
    pub fn skip_entry(&mut self) {
        self.idx.push(0);
        self.idx.push(0);
    }

    /// Finish the current entry WITHOUT adding a 0 terminator.
    /// Used for raw data entries where the caller has already written
    /// the complete byte sequence including the terminator.
    pub fn finish_entry_raw(&mut self) {
        let len = self.dat.len() - self.entry_start;
        self.idx.push((len >> 8) as u8);
        self.idx.push(len as u8);
    }

    pub fn p1(&mut self, value: u8) {
        self.dat.push(value);
    }

    pub fn p2(&mut self, value: u16) {
        self.dat.push((value >> 8) as u8);
        self.dat.push(value as u8);
    }

    pub fn p3(&mut self, value: u32) {
        self.dat.push((value >> 16) as u8);
        self.dat.push((value >> 8) as u8);
        self.dat.push(value as u8);
    }

    pub fn p4(&mut self, value: i32) {
        self.dat.push((value >> 24) as u8);
        self.dat.push((value >> 16) as u8);
        self.dat.push((value >> 8) as u8);
        self.dat.push(value as u8);
    }

    pub fn p2s(&mut self, value: i16) {
        self.p2(value as u16);
    }

    pub fn pbool(&mut self, value: bool) {
        self.p1(if value { 1 } else { 0 });
    }

    pub fn pjstr(&mut self, value: &str) {
        // Jagex strings are CP-1252, not UTF-8. Characters like à/é/ï are
        // single bytes in CP-1252 but 2 bytes in UTF-8; using as_bytes()
        // would produce output 1 byte too long per non-ASCII character.
        let encoded = rs_io::cp1252::encode_string(value);
        self.dat.extend_from_slice(&encoded);
        self.dat.push(0); // NUL terminator (JSTR format)
    }

    /// Merge another PackedData's entries into this one.
    ///
    /// For each entry ID, concatenates the opcode bytes from `other` before
    /// the terminating 0 of this entry. If only one side has data, uses that.
    /// Both PackedData must have the same count.
    ///
    /// Used to produce complete server .dat files that contain ALL opcodes
    /// (client rendering + server behavior) in a single entry per config.
    pub fn merge_from(&mut self, other: &PackedData) {
        let count = u16::from_be_bytes([self.dat[0], self.dat[1]]) as usize;
        let other_count = u16::from_be_bytes([other.dat[0], other.dat[1]]) as usize;
        assert_eq!(count, other_count, "merge_from: count mismatch");

        // Parse entry offsets from idx
        let parse_entries = |_dat: &[u8], idx: &[u8], count: usize| -> Vec<(usize, usize)> {
            let mut entries = Vec::with_capacity(count);
            let mut offset = 2usize; // skip p2(count) header
            for i in 0..count {
                let idx_off = 2 + i * 2;
                let len = u16::from_be_bytes([idx[idx_off], idx[idx_off + 1]]) as usize;
                entries.push((offset, len));
                offset += len;
            }
            entries
        };

        let self_entries = parse_entries(&self.dat, &self.idx, count);
        let other_entries = parse_entries(&other.dat, &other.idx, count);
        let old_dat = std::mem::take(&mut self.dat);

        let mut new_dat = Vec::with_capacity(old_dat.len() + other.dat.len());
        new_dat.push((count >> 8) as u8);
        new_dat.push(count as u8);

        let mut new_idx = Vec::with_capacity(2 + count * 2);
        new_idx.push((count >> 8) as u8);
        new_idx.push(count as u8);

        for i in 0..count {
            let (s_off, s_len) = self_entries[i];
            let (o_off, o_len) = other_entries[i];

            if s_len == 0 && o_len == 0 {
                new_idx.push(0);
                new_idx.push(0);
                continue;
            }

            let start = new_dat.len();

            // Write other's opcodes first (client data), stripping the terminating 0
            if o_len > 0 {
                let bytes = &other.dat[o_off..o_off + o_len];
                // Strip trailing 0 terminator
                let end = if bytes.last() == Some(&0) {
                    bytes.len() - 1
                } else {
                    bytes.len()
                };
                new_dat.extend_from_slice(&bytes[..end]);
            }

            // Write self's opcodes (server data), stripping the terminating 0
            if s_len > 0 {
                let bytes = &old_dat[s_off..s_off + s_len];
                let end = if bytes.last() == Some(&0) {
                    bytes.len() - 1
                } else {
                    bytes.len()
                };
                new_dat.extend_from_slice(&bytes[..end]);
            }

            // Add single terminator
            new_dat.push(0);

            let merged_len = new_dat.len() - start;
            new_idx.push((merged_len >> 8) as u8);
            new_idx.push(merged_len as u8);
        }

        self.dat = new_dat;
        self.idx = new_idx;
    }

    /// Save the .dat and .idx files to disk.
    pub fn save(&self, base_path: &Path, name: &str) -> Result<()> {
        std::fs::create_dir_all(base_path)?;
        let dat_path = base_path.join(format!("{name}.dat"));
        let idx_path = base_path.join(format!("{name}.idx"));
        std::fs::write(&dat_path, &self.dat)?;
        std::fs::write(&idx_path, &self.idx)?;
        info!(
            "  {name}: {count} entries, dat={dat_size} bytes, idx={idx_size} bytes",
            count = self.count,
            dat_size = self.dat.len(),
            idx_size = self.idx.len(),
        );
        Ok(())
    }
}
