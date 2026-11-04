//! RuneScape JS5/DAT2 cache store (rev 647 era).
//!
//! Read/write support for the dat2-generation cache used by `since_647`
//! targets, complementing `rs_io::jag`/`rs_io::js5` which cover the
//! pre-dat2 revisions:
//! - `main_file_cache.dat2` + `idx0..idx36` + `idx255` master index
//! - Group/entry container codecs (gzip, bzip2, lzma), trailing versions
//! - XTEA decryption for map regions (`keys.json`)
//! - Reference table parsing and assembly (CRC + version + whirlpool)

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use bytes::{Bytes, BytesMut};
use thiserror::Error;
use tracing::{debug, trace, warn};

pub mod archive;
pub mod compression;
pub mod map;
pub mod reference;
pub mod xtea;

use compression::decompress;

/// Cache error types
#[derive(Error, Debug)]
pub enum CacheError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid cache file format")]
    InvalidFormat,

    #[error("Archive {0} not found")]
    ArchiveNotFound(u8),

    #[error("Group {0} not found in archive {1}")]
    GroupNotFound(u32, u8),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("XTEA decryption failed")]
    XteaDecryption,

    #[error("CRC32 mismatch: expected {expected}, got {actual}")]
    CrcMismatch { expected: u32, actual: u32 },
}

pub type Result<T> = std::result::Result<T, CacheError>;

/// A cache store manages access to the RuneScape cache files
pub struct CacheStore {
    /// Path to the cache directory
    path: std::path::PathBuf,
    /// Open data file (main_file_cache.dat2)
    data_file: File,
    /// Open index files (main_file_cache.idx*)
    index_files: HashMap<u8, IndexFile>,
    /// Reference table cache
    reference_tables: HashMap<u8, reference::ReferenceTable>,
}

/// An index file header information
#[derive(Debug, Clone)]
struct IndexFile {
    /// Index ID (0-255)
    #[allow(dead_code)]
    id: u8,
    /// Number of groups in this archive
    group_count: u32,
}

/// A group entry in the cache
#[derive(Debug, Clone)]
pub struct Group {
    /// Group ID
    pub id: u32,
    /// Group name hash (CRC32 of name, or -1 if no name)
    pub name_hash: i32,
    /// CRC32 checksum
    pub crc: u32,
    /// Whirlpool hash (64 bytes)
    pub whirlpool: Option<[u8; 64]>,
    /// Version number
    pub version: u32,
    /// Decompressed data size
    pub size: usize,
    /// Entries within this group
    pub entries: Vec<Entry>,
}

/// An individual entry within a group
#[derive(Debug, Clone)]
pub struct Entry {
    /// Entry ID
    pub id: u32,
    /// Entry name hash
    pub name_hash: i32,
    /// Entry data
    pub data: Bytes,
}

impl CacheStore {
    /// Open a cache store from the given directory path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Open the data file
        let data_path = path.join("main_file_cache.dat2");
        let data_file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&data_path)?;

        // Open index files
        let mut index_files = HashMap::new();
        for i in 0..=255u8 {
            let idx_path = path.join(format!("main_file_cache.idx{i}"));
            if idx_path.exists() {
                let file = File::open(&idx_path)?;
                let metadata = file.metadata()?;
                let group_count = (metadata.len() / 6) as u32;

                index_files.insert(i, IndexFile { id: i, group_count });
                debug!("Loaded index file {} with {} groups", i, group_count);
            }
        }

        if index_files.is_empty() {
            return Err(CacheError::InvalidFormat);
        }

        let mut store = Self {
            path,
            data_file,
            index_files,
            reference_tables: HashMap::new(),
        };

        // Load reference tables
        store.load_reference_tables()?;

        Ok(store)
    }

    /// Load all reference tables from index 255
    fn load_reference_tables(&mut self) -> Result<()> {
        // Index 255 contains reference tables for other archives
        for archive_id in 0..=255u8 {
            match self.read_group(255, archive_id as u32) {
                Ok(Some(group)) => {
                    if let Some(entry) = group.entries.first() {
                        match reference::parse_reference_table(&entry.data) {
                            Ok(table) => {
                                self.reference_tables.insert(archive_id, table);
                                debug!("Loaded reference table for archive {}", archive_id);
                            }
                            Err(e) => {
                                trace!(
                                    "Could not parse reference table for archive {}: {}",
                                    archive_id, e
                                );
                            }
                        }
                    }
                }
                Ok(None) => {} // Group doesn't exist
                Err(e) => {
                    trace!(
                        "Could not read reference table for archive {}: {}",
                        archive_id, e
                    );
                }
            }
        }

        Ok(())
    }

    /// Read a raw group from the cache (before decompression)
    fn read_raw_group(&mut self, archive_id: u8, group_id: u32) -> Result<Option<Bytes>> {
        let index = self
            .index_files
            .get(&archive_id)
            .ok_or(CacheError::ArchiveNotFound(archive_id))?;

        if group_id >= index.group_count {
            return Ok(None);
        }

        // Read index entry (6 bytes: 3 byte size, 3 byte sector)
        let idx_path = self.path.join(format!("main_file_cache.idx{archive_id}"));
        let mut idx_file = File::open(&idx_path)?;
        idx_file.seek(SeekFrom::Start(group_id as u64 * 6))?;

        let mut idx_entry = [0u8; 6];
        idx_file.read_exact(&mut idx_entry)?;

        let size = ((idx_entry[0] as u32) << 16 | (idx_entry[1] as u32) << 8 | idx_entry[2] as u32)
            as usize;
        let sector = (idx_entry[3] as u32) << 16 | (idx_entry[4] as u32) << 8 | idx_entry[5] as u32;

        // Null entry: group doesn't exist in the cache
        if size == 0 || sector == 0 {
            return Ok(None);
        }

        trace!(
            "Reading group {} from archive {}, size={}, sector={}",
            group_id, archive_id, size, sector
        );

        // Sector layout: 520 bytes per sector
        // For group_id < 65536 (standard): 8-byte header + 512-byte data
        //   header: [2-byte group_id][2-byte chunk][3-byte next_sector][1-byte archive_id]
        // For group_id >= 65536 (extended): 10-byte header + 510-byte data
        let extended = group_id >= 65536;
        let header_len = if extended { 10 } else { 8 };
        let data_per_sector = 520 - header_len;

        let mut data = BytesMut::with_capacity(size);
        let mut current_sector = sector as u64;
        let mut remaining = size;
        let mut chunk = 0u16;

        while remaining > 0 {
            let sector_size = std::cmp::min(remaining, data_per_sector);
            let mut sector_data = vec![0u8; sector_size];

            self.data_file.seek(SeekFrom::Start(current_sector * 520))?;

            let mut header = [0u8; 10];
            self.data_file
                .read_exact(&mut header[..header_len])
                .map_err(|e| {
                    warn!(
                        "Failed to read sector header for group {} archive {}: {}",
                        group_id, archive_id, e
                    );
                    e
                })?;

            let (header_group, next_sector) = if extended {
                let g = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
                let n = ((header[6] as u32) << 16) | ((header[7] as u32) << 8) | header[8] as u32;
                (g, n as u64)
            } else {
                let g = u16::from_be_bytes([header[0], header[1]]) as u32;
                let n = ((header[4] as u32) << 16) | ((header[5] as u32) << 8) | header[6] as u32;
                (g, n as u64)
            };

            if header_group != group_id {
                warn!(
                    "Group ID mismatch in sector: expected {}, got {}",
                    group_id, header_group
                );
                return Err(CacheError::InvalidFormat);
            }

            self.data_file.read_exact(&mut sector_data)?;
            data.extend_from_slice(&sector_data);

            remaining -= sector_size;
            current_sector = next_sector;
            chunk += 1;
        }

        let _ = chunk; // used for validation, suppresses warning

        Ok(Some(data.freeze()))
    }

    /// Read and decompress a group from the cache, splitting into entries using the reference table
    pub fn read_group(&mut self, archive_id: u8, group_id: u32) -> Result<Option<Group>> {
        let Some(raw_data) = self.read_raw_group(archive_id, group_id)? else {
            return Ok(None);
        };

        // Decompress the data
        let decompressed = decompress(&raw_data)?;

        // Look up group info from reference table
        let group_info = self
            .reference_tables
            .get(&archive_id)
            .and_then(|rt| rt.groups.iter().find(|g| g.id == group_id))
            .cloned();

        let (name_hash, crc, whirlpool, version, entries) = if let Some(info) = group_info {
            let file_count = info.files.len();
            let entries = if file_count <= 1 {
                // Single file - return data directly
                let file_id = info.files.first().map(|f| f.id).unwrap_or(0);
                let file_name = info.files.first().and_then(|f| f.name_hash).unwrap_or(-1);
                vec![Entry {
                    id: file_id,
                    name_hash: file_name,
                    data: decompressed.clone(),
                }]
            } else {
                // Multi-file: split using stripe format
                split_group_entries(&decompressed, &info.files)
            };
            (
                info.name_hash,
                info.crc,
                info.whirlpool,
                info.version,
                entries,
            )
        } else {
            // No reference table - return as single entry
            let entries = vec![Entry {
                id: 0,
                name_hash: -1,
                data: decompressed.clone(),
            }];
            (-1, 0, None, 0, entries)
        };

        Ok(Some(Group {
            id: group_id,
            name_hash,
            crc,
            whirlpool,
            version,
            size: decompressed.len(),
            entries,
        }))
    }

    /// Get the number of groups in an archive
    pub fn group_count(&self, archive_id: u8) -> Option<u32> {
        self.index_files.get(&archive_id).map(|idx| idx.group_count)
    }

    /// Check if an archive exists
    pub fn has_archive(&self, archive_id: u8) -> bool {
        self.index_files.contains_key(&archive_id)
    }

    /// Get the reference table for an archive
    pub fn reference_table(&self, archive_id: u8) -> Option<&reference::ReferenceTable> {
        self.reference_tables.get(&archive_id)
    }

    /// Read and decompress a group, returning the raw decompressed bytes (before entry splitting).
    /// Useful for debugging cache format issues.
    pub fn read_group_raw_decompressed(
        &mut self,
        archive_id: u8,
        group_id: u32,
    ) -> Result<Option<Bytes>> {
        let Some(raw) = self.read_raw_group(archive_id, group_id)? else {
            return Ok(None);
        };
        Ok(Some(decompress(&raw)?))
    }

    /// Read the raw (compressed) bytes of a group for JS5 serving.
    ///
    /// Returns the exact bytes that the client expects: the compression header
    /// and compressed payload, as stored on disk.
    pub fn read_raw_group_bytes(&mut self, archive_id: u8, group_id: u32) -> Result<Option<Bytes>> {
        self.read_raw_group(archive_id, group_id)
    }

    /// Read a group, decrypt with XTEA, and decompress.
    ///
    /// Map landscape groups (`l_X_Z` in archive 5) are XTEA-encrypted.
    /// The encryption covers the compressed payload (bytes after the
    /// 5-byte compression header: type + compressed_len).
    ///
    /// Keys are i32 (as stored in OpenRS2 keys.json) and converted to u32
    /// for the cipher.
    /// Write raw data to a group in the cache.
    /// Appends sectors to the end of the dat file and updates the idx entry.
    /// The data should include the 2-byte version suffix if applicable.
    pub fn write_raw_group(&mut self, archive_id: u8, group_id: u32, data: &[u8]) -> Result<()> {
        use std::io::Write;

        let data_len = data.len();

        // Find end of dat file
        self.data_file.seek(SeekFrom::End(0))?;
        let dat_end = self.data_file.stream_position()?;
        let mut next_sector = (dat_end / 520) as u32;
        if dat_end % 520 != 0 {
            next_sector += 1;
        }
        if next_sector == 0 {
            next_sector = 1;
        }

        let first_sector = next_sector;
        let extended = group_id >= 65536;
        let header_len: usize = if extended { 10 } else { 8 };
        let data_per_sector: usize = 520 - header_len;

        let mut remaining = data_len;
        let mut data_offset: usize = 0;
        let mut chunk: u16 = 0;
        let mut current_sector = next_sector;

        while remaining > 0 {
            let write_len = remaining.min(data_per_sector);
            let is_last = remaining <= data_per_sector;
            let next = if is_last { 0u32 } else { current_sector + 1 };

            let mut sector = vec![0u8; 520];
            if extended {
                sector[0..4].copy_from_slice(&group_id.to_be_bytes());
                sector[4..6].copy_from_slice(&chunk.to_be_bytes());
                sector[6] = ((next >> 16) & 0xFF) as u8;
                sector[7] = ((next >> 8) & 0xFF) as u8;
                sector[8] = (next & 0xFF) as u8;
                sector[9] = archive_id;
            } else {
                sector[0] = ((group_id >> 8) & 0xFF) as u8;
                sector[1] = (group_id & 0xFF) as u8;
                sector[2] = ((chunk >> 8) & 0xFF) as u8;
                sector[3] = (chunk & 0xFF) as u8;
                sector[4] = ((next >> 16) & 0xFF) as u8;
                sector[5] = ((next >> 8) & 0xFF) as u8;
                sector[6] = (next & 0xFF) as u8;
                sector[7] = archive_id;
            }

            sector[header_len..header_len + write_len]
                .copy_from_slice(&data[data_offset..data_offset + write_len]);

            self.data_file
                .seek(SeekFrom::Start(current_sector as u64 * 520))?;
            self.data_file.write_all(&sector)?;

            remaining -= write_len;
            data_offset += write_len;
            chunk += 1;
            current_sector += 1;
        }

        // Update idx entry
        let idx_path = self.path.join(format!("main_file_cache.idx{archive_id}"));
        let mut idx_file = std::fs::OpenOptions::new().write(true).open(&idx_path)?;
        let mut idx_entry = [0u8; 6];
        idx_entry[0] = ((data_len >> 16) & 0xFF) as u8;
        idx_entry[1] = ((data_len >> 8) & 0xFF) as u8;
        idx_entry[2] = (data_len & 0xFF) as u8;
        idx_entry[3] = ((first_sector >> 16) & 0xFF) as u8;
        idx_entry[4] = ((first_sector >> 8) & 0xFF) as u8;
        idx_entry[5] = (first_sector & 0xFF) as u8;
        idx_file.seek(SeekFrom::Start(group_id as u64 * 6))?;
        idx_file.write_all(&idx_entry)?;

        Ok(())
    }

    /// Write a group to the cache with compression.
    /// Compresses the decompressed data, appends the 2-byte version suffix,
    /// writes to dat2, and updates idx.
    ///
    /// Returns the CRC32 of the compressed data (excluding the version suffix),
    /// which is what the reference table expects.
    pub fn write_group(
        &mut self,
        archive_id: u8,
        group_id: u32,
        data: &[u8],
        compression: compression::CompressionType,
        version: u16,
    ) -> Result<u32> {
        let compressed = compression::compress(data, compression)?;
        let crc = compute_group_crc(&compressed);
        // Append 2-byte version suffix (big-endian) — the idx size includes this
        let mut with_version = compressed;
        with_version.push((version >> 8) as u8);
        with_version.push(version as u8);
        self.write_raw_group(archive_id, group_id, &with_version)?;
        Ok(crc)
    }

    /// Write a group using the optimal compression codec.
    ///
    /// Tries Gzip and Bzip2, picks whichever is smaller. Falls back to None
    /// only if `allow_uncompressed` is true and compression increases size.
    pub fn write_group_best(
        &mut self,
        archive_id: u8,
        group_id: u32,
        data: &[u8],
        version: u16,
        allow_uncompressed: bool,
    ) -> Result<u32> {
        let allow_bzip2 = archive::archive_format(archive_id)
            .map(|f| f.allow_bzip2)
            .unwrap_or(true);
        let forced = archive::codec_override(archive_id, group_id);
        let compressed = compression::compress_best(data, allow_uncompressed, allow_bzip2, forced)?;
        let crc = compute_group_crc(&compressed);
        let mut with_version = compressed;
        with_version.push((version >> 8) as u8);
        with_version.push(version as u8);
        self.write_raw_group(archive_id, group_id, &with_version)?;
        Ok(crc)
    }

    /// Write an updated reference table for an archive.
    /// Encodes the table, compresses with gzip, writes to archive 255.
    ///
    /// Note: archive 255 entries (reference tables) do NOT carry a 2-byte
    /// version suffix in rev 647 — the table's version is encoded inside
    /// the reference-table body (per format >= 6). Adding a suffix here
    /// causes the JS5 client to fail with `error_game_js5connect` because
    /// the trailing bytes don't match the expected layout.
    pub fn write_reference_table(
        &mut self,
        archive_id: u8,
        table: &reference::ReferenceTable,
    ) -> Result<()> {
        let encoded = reference::encode_reference_table(table);
        let compressed = compression::compress_best(&encoded, true, true, None)?;
        // No version suffix for archive 255 entries.
        self.write_raw_group(255, archive_id as u32, &compressed)?;
        // Update our in-memory copy
        self.reference_tables.insert(archive_id, table.clone());
        Ok(())
    }

    /// Write a raw (already-compressed) reference table to archive 255
    /// and update the in-memory cache. Use this for passthrough where
    /// you already have the exact bytes from the source cache.
    pub fn write_reference_table_raw(
        &mut self,
        archive_id: u8,
        raw_compressed: &[u8],
        table: reference::ReferenceTable,
    ) -> Result<()> {
        self.write_raw_group(255, archive_id as u32, raw_compressed)?;
        self.reference_tables.insert(archive_id, table);
        Ok(())
    }

    pub fn read_group_xtea(
        &mut self,
        archive_id: u8,
        group_id: u32,
        keys: &[i32; 4],
    ) -> Result<Option<Bytes>> {
        let Some(raw) = self.read_raw_group(archive_id, group_id)? else {
            return Ok(None);
        };

        // If keys are all zero, skip decryption (unencrypted group).
        let all_zero = keys.iter().all(|&k| k == 0);
        if all_zero {
            return Ok(Some(decompress(&raw)?));
        }

        // The first 5 bytes are the compression header (type:1 + compressed_len:4).
        // XTEA encryption covers from byte 5 onward.
        if raw.len() < 5 {
            return Err(CacheError::InvalidFormat);
        }

        // Strip 2-byte version suffix before decryption (same as JS5 serving).
        let raw_len = raw.len();
        let mut buf = raw.to_vec();
        if raw_len >= 7 {
            buf.truncate(raw_len - 2);
        }
        let encrypted_section = &mut buf[5..];
        // Round down to 8-byte boundary for XTEA block cipher.
        let block_len = (encrypted_section.len() >> 3) << 3;
        if block_len > 0 {
            let u32_keys: [u32; 4] = [
                keys[0] as u32,
                keys[1] as u32,
                keys[2] as u32,
                keys[3] as u32,
            ];
            compression::Xtea::decrypt(&mut encrypted_section[..block_len], &u32_keys)
                .map_err(|_| CacheError::XteaDecryption)?;
        }

        Ok(Some(decompress(&buf)?))
    }
}

/// Split a multi-file group's decompressed data into individual entries using the JS5 stripe format.
///
/// The trailer at the end of the data contains:
/// - `num_stripes * file_count` big-endian i32 delta-encoded chunk sizes
///   (row-major: stripe 0 all files, stripe 1 all files, ...)
///   Each value is delta-encoded: actual_size = running_sum_per_stripe += read_i32()
/// - 1 byte: num_stripes
///
/// Data body is laid out as sequential stripes, each stripe containing one chunk per file in order.
///
/// Public so a group obtained from somewhere other than a local `CacheStore`
/// -- an archive server, say -- can be split with the same code the cache
/// reader uses. The stripe layout is fiddly enough that a second
/// implementation would be a second set of bugs.
pub fn split_group_entries(data: &Bytes, files: &[reference::FileInfo]) -> Vec<Entry> {
    let file_count = files.len();
    if data.is_empty() || file_count == 0 {
        return vec![];
    }

    let num_stripes = data[data.len() - 1] as usize;
    let trailer_size = 1 + num_stripes * file_count * 4;

    if num_stripes == 0 || data.len() < trailer_size {
        // Fallback: return all data as the single (first) file
        return vec![Entry {
            id: files[0].id,
            name_hash: files[0].name_hash.unwrap_or(-1),
            data: data.clone(),
        }];
    }

    let body_len = data.len() - trailer_size;
    let trailer_start = body_len;

    // Read chunk sizes: row-major [stripe][file], delta-encoded per stripe
    // Each i32 value is a delta: running_sum += delta gives the actual chunk size
    let mut chunk_sizes = vec![vec![0usize; file_count]; num_stripes];
    let mut trailer_pos = trailer_start;
    for stripe_row in chunk_sizes.iter_mut().take(num_stripes) {
        let mut prev_len: i32 = 0;
        for chunk_size in stripe_row.iter_mut().take(file_count) {
            let delta = i32::from_be_bytes([
                data[trailer_pos],
                data[trailer_pos + 1],
                data[trailer_pos + 2],
                data[trailer_pos + 3],
            ]);
            prev_len += delta;
            *chunk_size = prev_len.max(0) as usize;
            trailer_pos += 4;
        }
    }

    // Compute stripe start offsets
    let mut stripe_offsets = vec![0usize; num_stripes + 1];
    for stripe_idx in 0..num_stripes {
        let stripe_size: usize = chunk_sizes[stripe_idx].iter().sum();
        stripe_offsets[stripe_idx + 1] = stripe_offsets[stripe_idx] + stripe_size;
    }

    // Reconstruct each file by collecting its chunk from each stripe
    let body = &data[..body_len];
    let mut entries = Vec::with_capacity(file_count);

    for (file_idx, file_info) in files.iter().enumerate() {
        let total_size: usize = (0..num_stripes).map(|s| chunk_sizes[s][file_idx]).sum();
        let mut file_data = Vec::with_capacity(total_size);

        for stripe_idx in 0..num_stripes {
            let stripe_start = stripe_offsets[stripe_idx];
            let offset_in_stripe: usize = (0..file_idx).map(|f| chunk_sizes[stripe_idx][f]).sum();
            let chunk_size = chunk_sizes[stripe_idx][file_idx];
            let start = stripe_start + offset_in_stripe;
            if start + chunk_size <= body.len() {
                file_data.extend_from_slice(&body[start..start + chunk_size]);
            } else {
                warn!(
                    "Chunk out of bounds: file={}, stripe={}",
                    file_idx, stripe_idx
                );
            }
        }

        entries.push(Entry {
            id: file_info.id,
            name_hash: file_info.name_hash.unwrap_or(-1),
            data: Bytes::from(file_data),
        });
    }

    entries
}

/// Encode multiple file entries into a single group blob using the JS5 stripe format.
///
/// Each entry is `(file_id, data)`. For single-entry groups, returns the
/// data directly (no trailer). For multi-entry groups, produces a 1-stripe layout.
pub fn encode_group_entries(entries: &[(u32, &[u8])]) -> Vec<u8> {
    encode_group_entries_striped(entries, 1, |data, _| vec![data.to_vec()])
}

/// Encode with a custom stripe splitter.
///
/// `split_fn(entry_data, stripe_count) -> Vec<Vec<u8>>` splits each entry
/// into `stripe_count` chunks. The body interleaves all entries' chunk 0,
/// then all entries' chunk 1, etc.
pub fn encode_group_entries_striped(
    entries: &[(u32, &[u8])],
    num_stripes: u8,
    split_fn: impl Fn(&[u8], u8) -> Vec<Vec<u8>>,
) -> Vec<u8> {
    if entries.is_empty() {
        return vec![];
    }
    if entries.len() == 1 {
        return entries[0].1.to_vec();
    }

    let ns = num_stripes as usize;

    // Split each entry into stripes
    let split: Vec<Vec<Vec<u8>>> = entries
        .iter()
        .map(|(_, data)| split_fn(data, num_stripes))
        .collect();

    // Interleave: body = [all stripe 0 chunks][all stripe 1 chunks]...
    let mut body = Vec::new();
    for s in 0..ns {
        for entry_chunks in &split {
            body.extend_from_slice(&entry_chunks[s]);
        }
    }

    // Trailer: delta-encoded sizes per stripe, then num_stripes byte
    for s in 0..ns {
        let mut prev: i32 = 0;
        for entry_chunks in &split {
            let size = entry_chunks[s].len() as i32;
            body.extend_from_slice(&(size - prev).to_be_bytes());
            prev = size;
        }
    }
    body.push(num_stripes);
    body
}

/// Detect the number of files in a group blob by parsing its entry footer.
///
/// Multi-entry groups end with `[delta_sizes: file_count * num_stripes * 4 bytes] [num_stripes: u8]`.
/// Single-entry groups have no footer (the blob IS the data).
///
/// Returns 1 for single-entry blobs, or the detected file_count for multi-entry blobs.
pub fn detect_file_count(blob: &[u8]) -> u32 {
    if blob.is_empty() {
        return 1;
    }
    let num_stripes = blob[blob.len() - 1] as usize;
    if num_stripes == 0 {
        return 1;
    }
    // Try candidate file_counts from 2 upward.
    // trailer = file_count * num_stripes * 4 + 1
    // body = blob.len() - trailer
    // The accumulated deltas (per stripe) must sum to body length.
    for fc in 2u32..=4096 {
        let trailer_len = fc as usize * num_stripes * 4 + 1;
        if trailer_len >= blob.len() {
            break;
        }
        let body_len = blob.len() - trailer_len;
        let trailer_start = body_len;

        let mut total_size: i64 = 0;
        let mut valid = true;
        for stripe in 0..num_stripes {
            let mut prev: i32 = 0;
            for file in 0..fc as usize {
                let off = trailer_start + (stripe * fc as usize + file) * 4;
                let delta =
                    i32::from_be_bytes([blob[off], blob[off + 1], blob[off + 2], blob[off + 3]]);
                prev = prev.wrapping_add(delta);
                if prev < 0 {
                    valid = false;
                    break;
                }
                total_size += prev as i64;
            }
            if !valid {
                break;
            }
        }
        if valid && total_size == body_len as i64 {
            return fc;
        }
    }
    1
}

/// Compute CRC32 of a group's raw compressed bytes (for reference table updates).
pub fn compute_group_crc(raw_compressed: &[u8]) -> u32 {
    rs_io::crc::getcrc(raw_compressed, 0, raw_compressed.len()) as u32
}

/// Compress group data and compute CRC, returning the raw bytes ready for `write_raw_group`.
///
/// This is the thread-safe portion of `write_group_best` — it does the expensive
/// compression + CRC work without needing `&mut CacheStore`. Call this in parallel,
/// then feed results into `write_raw_group` sequentially.
pub fn prepare_group(
    archive_id: u8,
    group_id: u32,
    data: &[u8],
    version: u16,
    allow_uncompressed: bool,
) -> Result<(Vec<u8>, u32)> {
    let allow_bzip2 = archive::archive_format(archive_id)
        .map(|f| f.allow_bzip2)
        .unwrap_or(true);
    let forced = archive::codec_override(archive_id, group_id);
    let compressed = compression::compress_best(data, allow_uncompressed, allow_bzip2, forced)?;
    let crc = compute_group_crc(&compressed);
    let mut with_version = compressed;
    with_version.push((version >> 8) as u8);
    with_version.push(version as u8);
    Ok((with_version, crc))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_error_display() {
        let err = CacheError::InvalidFormat;
        assert_eq!(err.to_string(), "Invalid cache file format");
    }
}
