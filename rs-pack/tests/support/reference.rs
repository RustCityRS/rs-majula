use anyhow::{Context, Result};
use rs_io::js5::compression::decompress;
use rs_io::js5::reference::{ReferenceTable, parse_reference_table};
use std::collections::BTreeMap;

/// Decompress and parse a raw `(255, a)` reference-table container.
pub fn parse_reference_container(raw: &[u8]) -> Result<ReferenceTable> {
    let data = decompress(raw).context("decompressing reference table")?;
    parse_reference_table(&data).context("parsing reference table")
}

/// How two reference tables of the same archive disagree, group by group.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TableDiff {
    /// Groups present only in ours.
    pub only_ours: Vec<u32>,
    /// Groups present only in OpenRS2's.
    pub only_theirs: Vec<u32>,
    /// Groups in both whose CRC differs.
    pub crc: Vec<u32>,
    /// Groups in both whose CRC agrees but whose version does not.
    pub version: Vec<u32>,
    pub metadata: Vec<u32>,
    /// The table header (format, table version, named, whirlpooled) differs.
    pub header: bool,
}

impl TableDiff {
    /// Every group that differs in any way, sorted.
    pub fn groups(&self) -> Vec<u32> {
        let mut all: Vec<u32> = [
            &self.only_ours,
            &self.only_theirs,
            &self.crc,
            &self.version,
            &self.metadata,
        ]
        .into_iter()
        .flatten()
        .copied()
        .collect();
        all.sort_unstable();
        all.dedup();
        all
    }
}

pub fn diff_reference_tables(ours: &ReferenceTable, theirs: &ReferenceTable) -> TableDiff {
    let o: BTreeMap<_, _> = ours.groups.iter().map(|g| (g.id, g)).collect();
    let t: BTreeMap<_, _> = theirs.groups.iter().map(|g| (g.id, g)).collect();
    let mut diff = TableDiff {
        header: (ours.format, ours.version, ours.named, ours.whirlpooled)
            != (
                theirs.format,
                theirs.version,
                theirs.named,
                theirs.whirlpooled,
            ),
        ..TableDiff::default()
    };
    for (&id, ours) in &o {
        let Some(theirs) = t.get(&id) else {
            diff.only_ours.push(id);
            continue;
        };
        if ours.crc != theirs.crc {
            diff.crc.push(id);
        }
        if ours.version != theirs.version {
            diff.version.push(id);
        }
        if ours.name_hash != theirs.name_hash
            || ours.whirlpool != theirs.whirlpool
            || ours.file_count != theirs.file_count
            || !ours
                .files
                .iter()
                .map(|f| (f.id, f.name_hash))
                .eq(theirs.files.iter().map(|f| (f.id, f.name_hash)))
        {
            diff.metadata.push(id);
        }
    }
    diff.only_theirs = t.keys().filter(|id| !o.contains_key(id)).copied().collect();
    diff
}
