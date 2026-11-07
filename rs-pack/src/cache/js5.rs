use rs_io::js5::Result;
use rs_io::js5::compression::{decompress, decompress_xtea};
use rs_io::js5::reference::ReferenceTable;
use std::collections::HashMap;

#[derive(Default)]
pub struct PackedCache {
    pub groups: HashMap<(u8, u32), Vec<u8>>,
    pub tables: HashMap<u8, ReferenceTable>,
}

impl PackedCache {
    pub fn reference_table(&self, archive: u8) -> Option<&ReferenceTable> {
        self.tables.get(&archive)
    }

    pub fn read_group_raw_decompressed(&self, archive: u8, group: u32) -> Result<Option<Vec<u8>>> {
        self.groups
            .get(&(archive, group))
            .map(|data| decompress(data).map(|data| data.into_owned()))
            .transpose()
    }

    pub fn read_group_xtea(
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
                    decompress_xtea(data, keys)
                }
            })
            .transpose()
    }

    pub fn extend(&mut self, other: Self) {
        self.groups.extend(other.groups);
        self.tables.extend(other.tables);
    }
}
