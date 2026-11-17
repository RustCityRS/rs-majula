use rs_io::js5::Result;
use rs_io::js5::compression::{decompress, decompress_tiny_key};
use rs_io::js5::index::Js5Index;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct PackedCache {
    pub groups: HashMap<(u8, u32), Vec<u8>>,
    pub tables: HashMap<u8, Js5Index>,
    pub absent: HashSet<(u8, u32)>,
}

impl PackedCache {
    pub fn index(&self, archive: u8) -> Option<&Js5Index> {
        self.tables.get(&archive)
    }

    pub fn read_group_raw_decompressed(&self, archive: u8, group: u32) -> Result<Option<Vec<u8>>> {
        self.groups
            .get(&(archive, group))
            .map(|data| decompress(data).map(|data| data.into_owned()))
            .transpose()
    }

    pub fn read_group_tiny_key(
        &self,
        archive: u8,
        group: u32,
        keys: &[i32; 4],
    ) -> Result<Option<Vec<u8>>> {
        self.groups
            .get(&(archive, group))
            .map(|data| {
                if *keys == [0; 4] {
                    decompress(data).map(|data| data.into_owned())
                } else {
                    decompress_tiny_key(data, keys)
                }
            })
            .transpose()
    }

    pub fn extend(&mut self, other: Self) {
        self.groups.extend(other.groups);
        self.tables.extend(other.tables);
        self.absent.extend(other.absent);
    }
}
