//! Cache decompression support
//!
//! Handles multiple compression formats used in RuneScape cache:
//! - None (raw data)
//! - Bzip2 (legacy)
//! - Gzip (standard)
//! - LZMA (rev 647+)

use bytes::Bytes;

use crate::{CacheError, Result};
use rs_io::bz2::{bz2_compress, bz2_decompress};
use rs_io::gz::{gz_compress_with, gz_decompress};

/// Compression type used in cache
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionType {
    None = 0,
    Bzip2 = 1,
    Gzip = 2,
    /// LZMA, used in later revisions (647+)
    Lzma = 3,
}

impl TryFrom<u8> for CompressionType {
    type Error = CacheError;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0 => Ok(CompressionType::None),
            1 => Ok(CompressionType::Bzip2),
            2 => Ok(CompressionType::Gzip),
            3 => Ok(CompressionType::Lzma),
            _ => Err(CacheError::Decompression(format!(
                "Unknown compression type: {}",
                value
            ))),
        }
    }
}

/// Decompress cache data
///
/// Cache data format:
/// - Byte 0: Compression type
/// - Bytes 1-4: Compressed length (big-endian, includes header)
/// - Bytes 5-8: Decompressed length (big-endian)
/// - Remaining: Compressed data
pub fn decompress(data: &[u8]) -> Result<Bytes> {
    if data.len() < 5 {
        return Err(CacheError::InvalidFormat);
    }

    let compression = CompressionType::try_from(data[0])?;
    let compressed_len = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;

    // Uncompressed (type 0): 5-byte header only, no decompressed_len field
    if compression == CompressionType::None {
        if data.len() < 5 + compressed_len {
            return Err(CacheError::InvalidFormat);
        }
        return Ok(Bytes::copy_from_slice(&data[5..5 + compressed_len]));
    }

    // Compressed types: 9-byte header (5 + 4-byte decompressed_len)
    if data.len() < 9 {
        return Err(CacheError::InvalidFormat);
    }
    let decompressed_len = u32::from_be_bytes([data[5], data[6], data[7], data[8]]) as usize;

    if data.len() < 9 + compressed_len {
        return Err(CacheError::InvalidFormat);
    }

    if decompressed_len > 64 * 1024 * 1024 {
        return Err(CacheError::Decompression(format!(
            "Decompressed length {} bytes is unreasonably large (corrupt XTEA?)",
            decompressed_len
        )));
    }

    let compressed_data = &data[9..9 + compressed_len];

    let result = match compression {
        CompressionType::None => unreachable!(),
        CompressionType::Bzip2 => decompress_bzip2(compressed_data, decompressed_len)?,
        CompressionType::Gzip => decompress_gzip(compressed_data, decompressed_len)?,
        CompressionType::Lzma => decompress_lzma(compressed_data, decompressed_len)?,
    };

    Ok(result)
}

fn decompress_bzip2(data: &[u8], expected_len: usize) -> Result<Bytes> {
    let output = bz2_decompress(data, expected_len, true, 0);

    if output.len() != expected_len {
        return Err(CacheError::Decompression(format!(
            "Bzip2 length mismatch: expected {}, got {}",
            expected_len,
            output.len()
        )));
    }

    Ok(Bytes::from(output))
}

fn decompress_gzip(data: &[u8], expected_len: usize) -> Result<Bytes> {
    let output = gz_decompress(data, expected_len);

    if output.len() != expected_len {
        return Err(CacheError::Decompression(format!(
            "Gzip length mismatch: expected {}, got {}",
            expected_len,
            output.len()
        )));
    }

    Ok(Bytes::from(output))
}

/// Decompress LZMA data
fn decompress_lzma(data: &[u8], expected_len: usize) -> Result<Bytes> {
    let mut input = std::io::Cursor::new(data);
    let mut output = Vec::with_capacity(expected_len);

    lzma_rs::lzma_decompress(&mut input, &mut output)
        .map_err(|e| CacheError::Decompression(format!("LZMA error: {}", e)))?;

    Ok(Bytes::from(output))
}

/// Compress data into cache container format.
///
/// Container format:
/// - Byte 0: Compression type
/// - Bytes 1-4: Compressed length (big-endian u32)
/// - Bytes 5-8: Decompressed length (big-endian u32)
/// - Bytes 9+: Compressed data
pub fn compress(data: &[u8], compression: CompressionType) -> Result<Vec<u8>> {
    match compression {
        CompressionType::None => compress_none(data),
        CompressionType::Gzip => compress_gzip(data),
        CompressionType::Bzip2 => compress_bzip2(data),
        CompressionType::Lzma => compress_lzma(data),
    }
}

fn compress_none(data: &[u8]) -> Result<Vec<u8>> {
    let compressed_len = data.len() as u32;
    let mut out = Vec::with_capacity(5 + data.len());
    out.push(CompressionType::None as u8);
    out.extend_from_slice(&compressed_len.to_be_bytes());
    out.extend_from_slice(data);
    Ok(out)
}

fn compress_gzip(data: &[u8]) -> Result<Vec<u8>> {
    let decompressed_len = data.len() as u32;
    let compressed = gz_compress_with(data, 6, 0);
    let compressed_len = compressed.len() as u32;

    let mut out = Vec::with_capacity(9 + compressed.len());
    out.push(CompressionType::Gzip as u8);
    out.extend_from_slice(&compressed_len.to_be_bytes());
    out.extend_from_slice(&decompressed_len.to_be_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

fn compress_bzip2(data: &[u8]) -> Result<Vec<u8>> {
    let decompressed_len = data.len() as u32;
    let compressed = bz2_compress(data, true);
    let compressed_len = compressed.len() as u32;

    let mut out = Vec::with_capacity(9 + compressed.len());
    out.push(CompressionType::Bzip2 as u8);
    out.extend_from_slice(&compressed_len.to_be_bytes());
    out.extend_from_slice(&decompressed_len.to_be_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

fn compress_lzma(data: &[u8]) -> Result<Vec<u8>> {
    let decompressed_len = data.len() as u32;
    let mut compressed = Vec::new();
    let mut input = std::io::Cursor::new(data);
    lzma_rs::lzma_compress(&mut input, &mut compressed)
        .map_err(|e| CacheError::Decompression(format!("LZMA compress error: {e}")))?;
    let compressed_len = compressed.len() as u32;

    let mut out = Vec::with_capacity(9 + compressed.len());
    out.push(CompressionType::Lzma as u8);
    out.extend_from_slice(&compressed_len.to_be_bytes());
    out.extend_from_slice(&decompressed_len.to_be_bytes());
    out.extend_from_slice(&compressed);
    Ok(out)
}

/// Compress data using the codec that produces the smallest output.
///
/// Algorithm (reverse-engineered from Jagex's rev 647 cache):
/// 1. Compress with both Gzip and Bzip2
/// 2. Pick whichever compressed payload is smaller (tiebreak: Bzip2 wins)
/// 3. If `allow_uncompressed` AND the compressed payload ≥ raw data size,
///    fall back to None (uncompressed)
///
/// `allow_uncompressed` is a per-archive property — some archives (maps,
/// jingles, configs) always compress even when it increases size, while
/// others (animations, vorbis, synth) fall back to None for small or
/// incompressible data.
pub fn compress_best(
    data: &[u8],
    allow_uncompressed: bool,
    allow_bzip2: bool,
    forced_codec: Option<CompressionType>,
) -> Result<Vec<u8>> {
    if let Some(codec) = forced_codec {
        let compressed = compress(data, codec)?;
        let payload = if codec == CompressionType::None {
            compressed.len() - 5
        } else {
            compressed.len() - 9
        };
        if allow_uncompressed && payload >= data.len() {
            return compress_none(data);
        }
        return Ok(compressed);
    }

    let gzip = compress_gzip(data)?;

    let (best_compressed, best_payload) = if allow_bzip2 {
        let bzip2 = compress_bzip2(data)?;
        let gzip_payload = gzip.len() - 9;
        let bzip2_payload = bzip2.len() - 9;
        if bzip2_payload <= gzip_payload {
            (bzip2, bzip2_payload)
        } else {
            (gzip, gzip_payload)
        }
    } else {
        let gzip_payload = gzip.len() - 9;
        (gzip, gzip_payload)
    };

    if allow_uncompressed && best_payload >= data.len() {
        return compress_none(data);
    }

    Ok(best_compressed)
}

/// XTEA decryption for map regions
pub struct Xtea;

impl Xtea {
    /// Number of rounds for XTEA (64 is standard, 32 for RuneScape)
    pub const ROUNDS: u32 = 32;

    /// Decrypt data using XTEA
    ///
    /// # Arguments
    /// * `data` - Encrypted data (must be multiple of 8 bytes)
    /// * `key` - 128-bit key as 4 u32 values
    pub fn decrypt(data: &mut [u8], key: &[u32; 4]) -> Result<()> {
        if !data.len().is_multiple_of(8) {
            return Err(CacheError::XteaDecryption);
        }

        let delta: u32 = 0x9E3779B9;

        for chunk in data.as_chunks_mut::<8>().0 {
            let mut v0 = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let mut v1 = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);

            let mut sum: u32 = delta.wrapping_mul(Self::ROUNDS);

            for _ in 0..Self::ROUNDS {
                // RS2 XTEA: key index from sum bits, sum decrement BETWEEN updates
                v1 = v1.wrapping_sub(
                    (key[((sum >> 11) & 3) as usize].wrapping_add(sum))
                        ^ ((v0.wrapping_shl(4) ^ v0.wrapping_shr(5)).wrapping_add(v0)),
                );
                sum = sum.wrapping_sub(delta);
                v0 = v0.wrapping_sub(
                    (key[(sum & 3) as usize].wrapping_add(sum))
                        ^ ((v1.wrapping_shl(4) ^ v1.wrapping_shr(5)).wrapping_add(v1)),
                );
            }

            chunk[0..4].copy_from_slice(&v0.to_be_bytes());
            chunk[4..8].copy_from_slice(&v1.to_be_bytes());
        }

        Ok(())
    }

    /// Encrypt data using XTEA
    pub fn encrypt(data: &mut [u8], key: &[u32; 4]) -> Result<()> {
        if !data.len().is_multiple_of(8) {
            return Err(CacheError::XteaDecryption);
        }

        let delta: u32 = 0x9E3779B9;

        for chunk in data.as_chunks_mut::<8>().0 {
            let mut v0 = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            let mut v1 = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);

            let mut sum: u32 = 0;

            for _ in 0..Self::ROUNDS {
                v0 = v0.wrapping_add(
                    (key[(sum & 3) as usize].wrapping_add(sum))
                        ^ ((v1.wrapping_shl(4) ^ v1.wrapping_shr(5)).wrapping_add(v1)),
                );
                sum = sum.wrapping_add(delta);
                v1 = v1.wrapping_add(
                    (key[((sum >> 11) & 3) as usize].wrapping_add(sum))
                        ^ ((v0.wrapping_shl(4) ^ v0.wrapping_shr(5)).wrapping_add(v0)),
                );
            }

            chunk[0..4].copy_from_slice(&v0.to_be_bytes());
            chunk[4..8].copy_from_slice(&v1.to_be_bytes());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compression_type_from_u8() {
        assert_eq!(
            CompressionType::try_from(0u8).unwrap(),
            CompressionType::None
        );
        assert_eq!(
            CompressionType::try_from(1u8).unwrap(),
            CompressionType::Bzip2
        );
        assert_eq!(
            CompressionType::try_from(2u8).unwrap(),
            CompressionType::Gzip
        );
        assert_eq!(
            CompressionType::try_from(3u8).unwrap(),
            CompressionType::Lzma
        );
        assert!(CompressionType::try_from(99u8).is_err());
    }

    #[test]
    fn test_xtea_roundtrip() {
        let key: [u32; 4] = [0x01234567, 0x89ABCDEF, 0xFEDCBA98, 0x76543210];
        let original = b"ABCDEFGH";
        let mut data = original.to_vec();

        Xtea::encrypt(&mut data, &key).unwrap();
        Xtea::decrypt(&mut data, &key).unwrap();

        assert_eq!(&data, original);
    }

    #[test]
    fn test_xtea_invalid_length() {
        let key: [u32; 4] = [0x01234567, 0x89ABCDEF, 0xFEDCBA98, 0x76543210];
        let mut data = vec![1u8, 2, 3, 4, 5]; // Not multiple of 8

        assert!(Xtea::decrypt(&mut data, &key).is_err());
    }

    #[test]
    fn test_compress_decompress_none() {
        let original = b"Hello, RuneScape cache!";
        let compressed = compress(original, CompressionType::None).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(&decompressed[..], original);
    }

    #[test]
    fn test_compress_decompress_gzip() {
        let original = b"Hello, RuneScape cache! This is gzip compressed data.";
        let compressed = compress(original, CompressionType::Gzip).unwrap();
        assert_eq!(compressed[0], CompressionType::Gzip as u8);
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(&decompressed[..], original);
    }

    #[test]
    fn test_compress_decompress_bzip2() {
        let original = b"Hello, RuneScape cache! This is bzip2 compressed data.";
        let compressed = compress(original, CompressionType::Bzip2).unwrap();
        assert_eq!(compressed[0], CompressionType::Bzip2 as u8);
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(&decompressed[..], original);
    }

    #[test]
    fn test_compress_decompress_lzma() {
        let original = b"Hello, RuneScape cache! This is LZMA compressed data.";
        let compressed = compress(original, CompressionType::Lzma).unwrap();
        assert_eq!(compressed[0], CompressionType::Lzma as u8);
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(&decompressed[..], original);
    }

    #[test]
    fn test_compress_decompress_large() {
        // 64KB of repeated data (compresses well)
        let original: Vec<u8> = (0..65536).map(|i| (i % 256) as u8).collect();
        for compression in [
            CompressionType::None,
            CompressionType::Gzip,
            CompressionType::Bzip2,
            CompressionType::Lzma,
        ] {
            let compressed = compress(&original, compression).unwrap();
            let decompressed = decompress(&compressed).unwrap();
            assert_eq!(
                &decompressed[..],
                &original[..],
                "Round-trip failed for {:?}",
                compression
            );
        }
    }

    /// Test that compressing real cache data from archive 2 reference table
    /// produces a valid container that decompresses back.
    #[test]
    fn test_compress_real_reference_table() {
        let cache_dir = std::path::Path::new("../cache");
        if !cache_dir.exists() {
            eprintln!("Skipping (no cache)");
            return;
        }
        let mut store = crate::CacheStore::open(cache_dir).unwrap();
        let raw = store.read_group_raw_decompressed(255, 2).unwrap().unwrap();

        // Re-compress with gzip
        let compressed = compress(&raw, CompressionType::Gzip).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(&decompressed[..], &raw[..]);
    }
}
