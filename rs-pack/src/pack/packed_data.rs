use crate::pack::pack_registry::PackId;
use anyhow::{Result, ensure};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;
use std::fs;
use std::path::Path;
use tracing::debug;

/// Bytes the entry count takes at the start of `dat` and `idx`: from 742 a
/// table can hold more entries than a short counts.
#[cfg(before_742)]
pub(crate) const COUNT_LEN: usize = 2;
#[cfg(since_742)]
pub(crate) const COUNT_LEN: usize = 4;

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
    ) -> Result<impl ExactSizeIterator<Item = (u32, &'a [u8])>> {
        ensure!(
            dat.len() >= COUNT_LEN && idx.len() >= COUNT_LEN,
            "Missing config count"
        );
        let count = read_count(dat);
        ensure!(
            dat[..COUNT_LEN] == idx[..COUNT_LEN],
            "Config data and index counts differ"
        );
        ensure!(
            idx.len() >= COUNT_LEN + (count << 1),
            "Truncated config index"
        );
        let lengths = idx[COUNT_LEN..COUNT_LEN + (count << 1)]
            .as_chunks::<2>()
            .0
            .iter();
        let length = |bytes: &[u8; 2]| Packet::from(bytes).g2() as usize;
        ensure!(
            COUNT_LEN + lengths.clone().map(length).sum::<usize>() <= dat.len(),
            "Truncated config data"
        );
        let mut offset = COUNT_LEN;
        Ok(lengths.enumerate().map(move |(id, bytes)| {
            let start = offset;
            offset += length(bytes);
            (id as u32, &dat[start..offset])
        }))
    }

    pub fn new(count: PackId) -> Self {
        let mut dat = Vec::with_capacity(500_000);
        let mut idx = Vec::with_capacity((count as usize + 1) * 2);
        dat.resize(COUNT_LEN, 0);
        #[cfg(before_530)]
        Packet::from(&mut dat).p2(count);
        #[cfg(all(since_530, before_742))]
        Packet::from(&mut dat).p2(count as u16);
        #[cfg(since_742)]
        Packet::from(&mut dat).p4(count as i32);
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
        let count = if dat.len() >= COUNT_LEN {
            read_count(&dat) as PackId
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
    pub fn pbigid(&mut self, value: TypeId) {
        #[cfg(before_742)]
        self.p2(value.g2());
        #[cfg(since_742)]
        self.packet(if (-1..32767).contains(&value.int()) {
            2
        } else {
            4
        })
        .psmartint2(value.int());
    }

    /// Writes a two-byte seq or graphic id as the client reads it.
    #[inline]
    pub fn pid(&mut self, value: u16) {
        self.pbigid(TypeId::from_g2(value));
    }

    #[inline]
    pub fn pjstr(&mut self, value: &str) {
        #[cfg(before_530)]
        {
            let mut buf = self.packet(value.len() + 1);
            buf.pdata(value.as_bytes(), 0, value.len());
            buf.p1(STRING_TERMINATOR);
        }
        #[cfg(since_530)]
        self.packet(Packet::sizejstr(value))
            .pjstr(value, STRING_TERMINATOR);
    }

    #[cfg(since_530)]
    pub fn pmidivarlen(&mut self, value: u32) {
        let start = self.dat.len();
        let mut buf = self.packet(5);
        buf.pmidivarlen(value);
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

pub(crate) fn read_count(dat: &[u8]) -> usize {
    #[cfg(before_742)]
    {
        Packet::from(dat).g2() as usize
    }
    #[cfg(since_742)]
    {
        Packet::from(dat).g4s() as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indexed_entries_preserve_gaps_and_check_lengths() {
        let count = &3u32.to_be_bytes()[4 - COUNT_LEN..];
        let dat = [count, &[0, 1, 7, 0]].concat();
        let idx = [count, &[0, 1, 0, 0, 0, 3]].concat();
        let c = COUNT_LEN;
        assert_eq!(
            PackedData::entries(&dat, &idx).unwrap().collect::<Vec<_>>(),
            vec![
                (0, &dat[c..c + 1]),
                (1, &dat[c + 1..c + 1]),
                (2, &dat[c + 1..c + 4])
            ]
        );
        assert!(PackedData::entries(&[], &idx).is_err());
        assert!(PackedData::entries(&dat, &idx[..c + 4]).is_err());
        assert!(PackedData::entries(&dat[..c + 3], &idx).is_err());
        assert!(PackedData::entries(&dat, &[0, 0]).is_err());
    }
}
