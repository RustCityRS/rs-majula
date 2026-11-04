//! Reference table parsing
//!
//! Reference tables (stored in archive 255) contain metadata about groups in other archives.

use bytes::Bytes;
use rs_io::Packet;
use tracing::trace;

use crate::Result;

/// A reference table containing metadata about an archive
#[derive(Debug, Clone)]
pub struct ReferenceTable {
    /// Archive ID
    pub archive_id: u8,
    /// Format version
    pub format: u8,
    /// Table version (format >= 6)
    pub version: u32,
    /// Whether entries have names
    pub named: bool,
    /// Whether entries have whirlpool hashes
    pub whirlpooled: bool,
    /// Groups in this archive
    pub groups: Vec<GroupInfo>,
}

/// Metadata about a group
#[derive(Debug, Clone)]
pub struct GroupInfo {
    /// Group ID
    pub id: u32,
    /// Name hash (CRC32 of name, or -1 if no name)
    pub name_hash: i32,
    /// CRC32 checksum
    pub crc: u32,
    /// Whirlpool hash (64 bytes, optional)
    pub whirlpool: Option<[u8; 64]>,
    /// Version/revision number
    pub version: u32,
    /// Decompressed size
    pub size: usize,
    /// Number of files in this group
    pub file_count: u32,
    /// File entries in this group
    pub files: Vec<FileInfo>,
}

/// Metadata about a file within a group
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File ID
    pub id: u32,
    /// Name hash (optional)
    pub name_hash: Option<i32>,
}

/// Read a "big smart" value: if the peek byte < 128, read a u16; otherwise
/// read an i32 and subtract 0x80000000.  The external `Packet` does not
/// provide this method, so it is inlined here.
fn gbigsmart(pkt: &mut Packet) -> i32 {
    if pkt.data[pkt.pos] < 128 {
        pkt.g2() as i32
    } else {
        pkt.g4s() - 0x80000000u32 as i32
    }
}

/// Parse a reference table from raw data.
///
/// Reference table format (all versions):
///   u8:  format version (5, 6, or 7)
///   u8:  flags  (0x01=named, 0x02=whirlpooled, 0x04=sizes)
///   u16: group count (format < 7)  OR  big_smart (format >= 7)
///   [group IDs — delta-encoded u16s (format < 7) or big_smarts (format >= 7)]
///   [name hashes — i32 per group, present when named flag set]
///   [CRC values — u32 per group]
///   [whirlpool hashes — 64 bytes per group, present when whirlpooled flag set]
///   [versions — u32 per group]
///   [file counts — u16 per group (format < 7) or big_smart (format >= 7)]
///   [file IDs per group — delta-encoded u16s or big_smarts]
///   [file name hashes per group — i32, present when named flag set]
pub fn parse_reference_table(data: &Bytes) -> Result<ReferenceTable> {
    let mut pkt = Packet::from(data.to_vec());

    let format = pkt.g1();

    // For format >= 6, a 4-byte version field precedes the flags byte
    let version = if format >= 6 { pkt.g4s() as u32 } else { 0 };

    // flags byte: 0x01=named, 0x02=whirlpooled, 0x04=sizes
    let flags = pkt.g1();
    let named = (flags & 0x01) != 0;
    let whirlpooled = (flags & 0x02) != 0;

    let group_count = if format >= 7 {
        gbigsmart(&mut pkt) as u32
    } else {
        pkt.g2() as u32
    };

    trace!(
        "Parsing reference table: format={}, flags={:#04x}, groups={}",
        format, flags, group_count
    );

    // Group IDs (delta-encoded)
    let mut group_ids = Vec::with_capacity(group_count as usize);
    let mut last_id = 0u32;
    for _ in 0..group_count {
        let delta = if format >= 7 {
            gbigsmart(&mut pkt) as u32
        } else {
            pkt.g2() as u32
        };
        last_id += delta;
        group_ids.push(last_id);
    }

    // Name hashes (if named flag)
    let group_names: Vec<i32> = if named {
        (0..group_count).map(|_| pkt.g4s()).collect()
    } else {
        vec![-1; group_count as usize]
    };

    // CRC values
    let group_crcs: Vec<u32> = (0..group_count).map(|_| pkt.g4s() as u32).collect();

    // Whirlpool hashes (if whirlpooled flag)
    let mut group_whirlpools: Vec<Option<[u8; 64]>> = vec![None; group_count as usize];
    if whirlpooled {
        for wp in group_whirlpools.iter_mut() {
            let mut hash = [0u8; 64];
            pkt.gdata(&mut hash, 0, 64);
            *wp = Some(hash);
        }
    }

    // Versions
    let group_versions: Vec<u32> = (0..group_count).map(|_| pkt.g4s() as u32).collect();

    // File counts per group
    let group_file_counts: Vec<u32> = (0..group_count)
        .map(|_| {
            if format >= 7 {
                gbigsmart(&mut pkt) as u32
            } else {
                pkt.g2() as u32
            }
        })
        .collect();

    // Build group structs
    let mut groups: Vec<GroupInfo> = (0..group_count as usize)
        .map(|i| GroupInfo {
            id: group_ids[i],
            name_hash: group_names[i],
            crc: group_crcs[i],
            whirlpool: group_whirlpools[i],
            version: group_versions[i],
            size: 0,
            file_count: group_file_counts[i],
            files: Vec::with_capacity(group_file_counts[i] as usize),
        })
        .collect();

    // File IDs per group (delta-encoded)
    for group in &mut groups {
        let fc = group.file_count as usize;
        let mut file_ids = Vec::with_capacity(fc);
        let mut last_file_id = 0u32;
        for _ in 0..fc {
            let delta = if format >= 7 {
                gbigsmart(&mut pkt) as u32
            } else {
                pkt.g2() as u32
            };
            last_file_id += delta;
            file_ids.push(last_file_id);
        }

        // File name hashes (if named flag)
        if named {
            for fid in file_ids {
                let name_hash = pkt.g4s();
                group.files.push(FileInfo {
                    id: fid,
                    name_hash: Some(name_hash),
                });
            }
        } else {
            for fid in file_ids {
                group.files.push(FileInfo {
                    id: fid,
                    name_hash: None,
                });
            }
        }
    }

    Ok(ReferenceTable {
        archive_id: 0, // Set by caller
        format,
        version,
        named,
        whirlpooled,
        groups,
    })
}

/// Write a psmart value into a `Vec<u8>`.
///
/// Same encoding as `Packet::psmart` but appends to a growable vec
/// instead of writing into a fixed-size buffer.
fn vec_psmart(out: &mut Vec<u8>, value: i32) {
    if (0..128).contains(&value) {
        out.push(value as u8);
    } else if (0..32768).contains(&value) {
        let v = (value + 32768) as u16;
        out.extend_from_slice(&v.to_be_bytes());
    } else {
        panic!("Error psmart out of range: {value}");
    }
}

/// Encode a reference table back to binary format.
pub fn encode_reference_table(table: &ReferenceTable) -> Vec<u8> {
    let mut out = Vec::with_capacity(4096);

    // Format version
    out.push(table.format);

    // Version field (format >= 6)
    if table.format >= 6 {
        out.extend_from_slice(&table.version.to_be_bytes());
    }

    // Flags
    let mut flags: u8 = 0;
    if table.named {
        flags |= 0x01;
    }
    if table.whirlpooled {
        flags |= 0x02;
    }
    out.push(flags);

    // Group count
    let gc = table.groups.len() as u32;
    if table.format >= 7 {
        vec_psmart(&mut out, gc as i32);
    } else {
        out.extend_from_slice(&(gc as u16).to_be_bytes());
    }

    // Group IDs (delta-encoded)
    let mut last_id = 0u32;
    for group in &table.groups {
        let delta = group.id - last_id;
        if table.format >= 7 {
            vec_psmart(&mut out, delta as i32);
        } else {
            out.extend_from_slice(&(delta as u16).to_be_bytes());
        }
        last_id = group.id;
    }

    // Name hashes
    if table.named {
        for group in &table.groups {
            out.extend_from_slice(&group.name_hash.to_be_bytes());
        }
    }

    // CRC values
    for group in &table.groups {
        out.extend_from_slice(&group.crc.to_be_bytes());
    }

    // Whirlpool hashes
    if table.whirlpooled {
        for group in &table.groups {
            if let Some(wp) = &group.whirlpool {
                out.extend_from_slice(wp);
            } else {
                out.extend_from_slice(&[0u8; 64]);
            }
        }
    }

    // Versions
    for group in &table.groups {
        out.extend_from_slice(&group.version.to_be_bytes());
    }

    // File counts
    for group in &table.groups {
        if table.format >= 7 {
            vec_psmart(&mut out, group.file_count as i32);
        } else {
            out.extend_from_slice(&(group.file_count as u16).to_be_bytes());
        }
    }

    // File IDs per group (delta-encoded)
    for group in &table.groups {
        let mut last_file_id = 0u32;
        for file in &group.files {
            let delta = file.id - last_file_id;
            if table.format >= 7 {
                vec_psmart(&mut out, delta as i32);
            } else {
                out.extend_from_slice(&(delta as u16).to_be_bytes());
            }
            last_file_id = file.id;
        }

        // File name hashes (0 = no name, matches Jagex default)
        if table.named {
            for file in &group.files {
                let hash = file.name_hash.unwrap_or(0);
                out.extend_from_slice(&hash.to_be_bytes());
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_packet_reads() {
        let mut pkt = Packet::from(vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07]);
        assert_eq!(pkt.g1(), 0x01);
        assert_eq!(pkt.g2(), 0x0203);
        assert_eq!(pkt.g4s() as u32, 0x04050607);
    }

    /// Round-trip test: decode reference table from real cache → encode → compare bytes.
    /// Verifies that our encoder produces byte-identical output to the original cache data.
    #[test]
    fn test_reference_table_roundtrip_archive2() {
        let cache_dir = std::path::Path::new("../cache");
        if !cache_dir.exists() {
            eprintln!("Skipping reference table round-trip test (no cache directory)");
            return;
        }

        let mut store = crate::CacheStore::open(cache_dir).expect("Failed to open cache");

        // Archive 2 = CONFIG — the one we'll be writing to
        let raw_decompressed = store
            .read_group_raw_decompressed(255, 2)
            .expect("Failed to read archive 2 reference table")
            .expect("Archive 2 reference table not found");

        // Parse
        let table = parse_reference_table(&raw_decompressed)
            .expect("Failed to parse archive 2 reference table");

        eprintln!(
            "Archive 2: format={} named={} whirlpooled={} groups={}",
            table.format,
            table.named,
            table.whirlpooled,
            table.groups.len()
        );

        // Encode back
        let re_encoded = encode_reference_table(&table);

        // Diagnostic: find first mismatch
        if raw_decompressed.len() != re_encoded.len() {
            eprintln!(
                "Length mismatch: original={} re-encoded={}",
                raw_decompressed.len(),
                re_encoded.len()
            );
        } else {
            for i in 0..raw_decompressed.len() {
                if raw_decompressed[i] != re_encoded[i] {
                    eprintln!(
                        "First byte mismatch at offset {}: original=0x{:02x} re-encoded=0x{:02x}",
                        i, raw_decompressed[i], re_encoded[i]
                    );
                    eprintln!(
                        "  original context: {:02x?}",
                        &raw_decompressed
                            [i.saturating_sub(4)..i.min(raw_decompressed.len()).saturating_add(8)]
                    );
                    eprintln!(
                        "  encoded context:  {:02x?}",
                        &re_encoded[i.saturating_sub(4)..i.min(re_encoded.len()).saturating_add(8)]
                    );
                    break;
                }
            }
        }

        // Compare byte-for-byte
        assert_eq!(
            &raw_decompressed[..],
            &re_encoded[..],
            "Reference table bytes don't match after round-trip"
        );
    }

    /// Round-trip ALL reference tables (archives 0-40) to verify the encoder
    /// handles every format variant in our cache.
    #[test]
    fn test_reference_table_roundtrip_all() {
        let cache_dir = std::path::Path::new("../cache");
        if !cache_dir.exists() {
            eprintln!("Skipping reference table round-trip test (no cache directory)");
            return;
        }

        let mut store = crate::CacheStore::open(cache_dir).expect("Failed to open cache");

        let mut tested = 0;
        let mut failed = Vec::new();
        for archive_id in 0..=40u8 {
            let Ok(Some(raw)) = store.read_group_raw_decompressed(255, archive_id as u32) else {
                continue;
            };
            let Ok(table) = parse_reference_table(&raw) else {
                continue;
            };
            let re_encoded = encode_reference_table(&table);
            if raw.len() != re_encoded.len() || raw[..] != re_encoded[..] {
                failed.push(archive_id);
            }
            tested += 1;
        }

        assert!(
            failed.is_empty(),
            "Reference table round-trip failed for archives: {:?} ({} of {} tested)",
            failed,
            failed.len(),
            tested
        );
        assert!(
            tested > 0,
            "No reference tables tested (cache may be empty)"
        );
        eprintln!("Successfully round-tripped {} reference tables", tested);
    }

    /// Full write pipeline test: copy cache → write reference table → read back → verify.
    #[test]
    fn test_write_reference_table_roundtrip() {
        let cache_dir = std::path::Path::new("../cache");
        if !cache_dir.exists() {
            eprintln!("Skipping write test (no cache directory)");
            return;
        }

        // Copy cache files to project-local temp dir so we don't corrupt the original
        let tmp = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../target/test_tmp/rs_cache_write_test");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        for entry in std::fs::read_dir(cache_dir).unwrap() {
            let entry = entry.unwrap();
            if !entry.file_type().unwrap().is_file() {
                continue;
            }
            let dest = tmp.join(entry.file_name());
            std::fs::copy(entry.path(), &dest).unwrap();
        }

        // Read original reference table for archive 2
        let mut store = crate::CacheStore::open(&tmp).expect("Failed to open temp cache");
        let original_raw = store.read_group_raw_decompressed(255, 2).unwrap().unwrap();
        let table = parse_reference_table(&original_raw).unwrap();

        // Write it back (compress + write to archive 255)
        store.write_reference_table(2, &table).unwrap();

        // Read it back and verify
        drop(store);
        let mut store2 = crate::CacheStore::open(&tmp).expect("Failed to reopen temp cache");
        let readback = store2.read_group_raw_decompressed(255, 2).unwrap().unwrap();

        assert_eq!(
            &original_raw[..],
            &readback[..],
            "Reference table changed after write-read round-trip"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        eprintln!("Write reference table round-trip: PASSED");
    }
}
