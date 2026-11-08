use crate::pack::pack_registry::PackId;
use anyhow::{Result, ensure};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use std::fs;
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
        let count = Packet::from(dat).g2() as usize;
        ensure!(dat[..2] == idx[..2], "Config data and index counts differ");
        ensure!(idx.len() >= 2 + (count << 1), "Truncated config index");
        let lengths = idx[2..2 + (count << 1)].as_chunks::<2>().0.iter();
        let length = |bytes: &[u8; 2]| Packet::from(bytes).g2() as usize;
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
        dat.resize(2, 0);
        #[cfg(before_647)]
        let header_count = count;
        #[cfg(since_647)]
        let header_count = count as u16;
        Packet::from(&mut dat).p2(header_count);
        idx.extend_from_slice(&dat);
        let entry_start = dat.len();
        Self {
            dat,
            idx,
            count,
            entry_start,
        }
    }

    /// Data an earlier pack made, from its `dat` and `idx`.
    pub fn from_parts(dat: Vec<u8>, idx: Vec<u8>) -> Self {
        let count = if dat.len() >= 2 {
            Packet::from(&dat[..2]).g2() as PackId
        } else {
            0
        };
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
        let pos = self.idx.len();
        self.idx.resize(pos + 2, 0);
        Packet::from(&mut self.idx[pos..]).p2(len as u16);
    }

    #[inline]
    pub fn skip_entry(&mut self) {
        self.idx.extend_from_slice(&[0, 0]);
    }

    #[inline]
    pub fn p1(&mut self, value: u8) {
        self.packet(1).p1(value);
    }

    #[inline]
    pub fn p2(&mut self, value: u16) {
        self.packet(2).p2(value);
    }

    #[inline]
    pub fn p3(&mut self, value: i32) {
        self.packet(3).p3(value);
    }

    #[inline]
    pub fn p4(&mut self, value: i32) {
        self.packet(4).p4(value);
    }

    #[inline]
    pub fn pjstr(&mut self, value: &str) {
        #[cfg(before_647)]
        {
            let mut buf = self.packet(value.len() + 1);
            buf.pdata(value.as_bytes(), 0, value.len());
            buf.p1(STRING_TERMINATOR);
        }
        #[cfg(since_647)]
        self.packet(Packet::sizejstr(value))
            .pjstr(value, STRING_TERMINATOR);
    }

    #[cfg(since_647)]
    pub fn pvarint(&mut self, value: u32) {
        let start = self.dat.len();
        let mut buf = self.packet(5);
        buf.pvarint(value);
        let len = buf.pos;
        self.dat.truncate(start + len);
    }

    fn packet(&mut self, len: usize) -> Packet<&mut [u8]> {
        let start = self.dat.len();
        self.dat.resize(start + len, 0);
        Packet::from(&mut self.dat[start..])
    }

    pub fn entry(&self) -> &[u8] {
        &self.dat[self.entry_start..]
    }

    /// Save the .dat and .idx files to disk.
    pub fn save(&self, base_path: &Path, name: &str) -> Result<()> {
        fs::create_dir_all(base_path)?;
        let dat_path = base_path.join(format!("{name}.dat"));
        let idx_path = base_path.join(format!("{name}.idx"));
        fs::write(&dat_path, &self.dat)?;
        fs::write(&idx_path, &self.idx)?;
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
