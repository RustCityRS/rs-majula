//! JS5 (cache serving) protocol
//!
//! Handles serving cache files to the client over the JS5 service connection.
//! The client requests groups by (archive, group) and the server streams back
//! the raw compressed bytes with 0xFF separator bytes every 512 data bytes.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use tracing::info;

use num_bigint::{BigInt, Sign};
use rs_crypto::rsa::RsaKey;
use rs_dat2::CacheStore;
use rs_io::Packet;

/// JS5 request from client
#[derive(Debug, Clone)]
pub struct Js5Request {
    /// Archive ID (0-36, or 255 for reference tables)
    pub archive: u8,
    /// Group ID within the archive
    pub group: u32,
    /// Urgent flag (client wants this ASAP, e.g. for the current map region)
    pub urgent: bool,
}

impl Js5Request {
    /// Parse from a 3-byte body: [archive][group_hi][group_lo]
    /// The opcode byte (0 = normal, 1 = urgent) is passed separately.
    pub fn parse(opcode: u8, data: &[u8]) -> Option<Self> {
        match opcode {
            0 | 1 => {
                if data.len() < 3 {
                    return None;
                }
                let mut req = Packet::from(data.to_vec());
                let archive = req.g1();
                let group = req.g2() as u32;
                Some(Self {
                    archive,
                    group,
                    urgent: opcode == 1,
                })
            }
            // opcode 2 = client settings/rekey, handled at call site
            _ => None,
        }
    }
}

/// JS5 response: raw cache group data sent back to the client.
///
/// Wire format:
/// ```text
/// [archive:1][group_hi:1][group_lo:1]
/// [data[0..512]]
/// [0xFF]               ← inserted before every 513th byte, 1025th byte, etc.
/// [data[512..1024]]
/// [0xFF]
/// ...
/// ```
#[derive(Debug, Clone)]
pub struct Js5Response {
    /// Archive ID
    pub archive: u8,
    /// Group ID
    pub group: u32,
    /// Raw compressed group data (as read from the .dat2 file)
    pub data: Bytes,
}

impl Js5Response {
    pub fn new(archive: u8, group: u32, data: Bytes) -> Self {
        Self {
            archive,
            group,
            data,
        }
    }

    /// Encode to bytes for transmission.
    ///
    /// Wire format (rev 647):
    ///   [archive:1][group:2][comp_type|priority_flag:1][size:4][payload[0..503]]
    ///   [0xFF]
    ///   [payload[504..1014]]
    ///   [0xFF]
    ///   ...
    ///
    /// A 0xFF separator byte is inserted before every output byte whose position
    /// (0-indexed from the archive byte) is a non-zero multiple of 512.
    /// The high bit (0x80) of the compression byte is set for non-urgent responses.
    /// Encode into `out`. Caller owns the Packet lifecycle (alloc/release).
    pub fn encode(&self, out: &mut rs_io::Packet, urgent: bool) {
        out.p1(self.archive);
        out.p2(self.group as u16);

        // Track window position exactly as the client does (and as the reference
        // server's toFilePayload does): starts at 3 (the 3-byte archive/group prefix
        // already counts toward the 512-byte window), and resets to 1 after each
        // 0xFF separator — NOT to 0. This keeps subsequent separator intervals at
        // exactly 511 data bytes, matching what the client expects.
        let mut window_pos: usize = 3;
        for (i, &byte) in self.data.iter().enumerate() {
            if window_pos == 512 {
                out.p1(0xFF);
                window_pos = 1;
            }
            // Patch the priority flag into the compression byte (byte 0 of data)
            if i == 0 && !urgent {
                out.p1(byte | 0x80);
            } else {
                out.p1(byte);
            }
            window_pos += 1;
        }
    }

    /// Estimate the encoded size: `3 + data + separators`.
    pub fn encoded_size(&self) -> usize {
        3 + self.data.len() + self.data.len() / 511 + 2
    }
}

/// JS5 cache: hot groups held in memory, everything else read from disk on demand.
///
/// Only archive 255 (reference tables) and the synthetic master index are
/// pre-loaded at startup so the server is ready in under a second.  All other
/// groups are fetched lazily through a shared `Mutex<CacheStore>`.
#[derive(Clone)]
pub struct Js5Cache {
    /// Pre-loaded hot groups (archive 255 reference tables + master index).
    hot: Arc<HashMap<(u8, u32), Bytes>>,
    /// On-demand access to the raw cache files (behind a mutex for &mut seek).
    store: Arc<Mutex<CacheStore>>,
}

impl Js5Cache {
    /// Initialise the JS5 cache.
    ///
    /// Pre-loads only archive 255 (reference tables) and builds the signed
    /// master index blob.  All other groups are served from disk on request.
    ///
    /// `private_key` is used to RSA-sign the master index (archive 255, group 255).
    pub fn preload(cache: CacheStore, private_key: &RsaKey) -> anyhow::Result<Self> {
        // We need a mut reference to read groups, then we store the cache.
        let mut cache = cache;
        let mut hot: HashMap<(u8, u32), Bytes> = HashMap::new();

        // Determine how many archives exist from the idx255 entry count.
        let archive_count = cache.group_count(255).unwrap_or(27) as u8;

        // Synthetic empty reference table for archives with no data in the cache.
        // Decompressed content: [format=5:1][flags=1:1][group_count=0:2 (u16 BE)]
        // Wrapped in cache container (no compression): [comp=0:1][size=4:4][data:4]
        // Format 5 requires: flags byte, then group count (u16 BE, NOT smart-int).
        // Flags bit 0 (names) MUST be set so that the name lookup table (aClass195_1)
        // is initialized — otherwise Class400.method9102 NPEs on name-based lookups.
        // With 0 groups, the name table is simply empty (returns -1 for all lookups).
        // Version check passes because format<6 sets anInt7569=0, matching master-index version=0.
        let empty_ref_table = Bytes::from_static(&[0u8, 0, 0, 0, 4, 5, 1, 0, 0]);

        // Pre-load archive 255 reference tables — these are needed immediately
        // for the master index CRCs and are small in total.
        // Null IDX entries (size=0, sector=0) get the synthetic empty reference table so that
        // the CRC/whirlpool in the master index match what we actually serve.
        for archive_id in 0u8..archive_count {
            let data = match cache.read_raw_group_bytes(255, archive_id as u32) {
                Ok(Some(data)) => data,
                _ => empty_ref_table.clone(),
            };
            hot.insert((255, archive_id as u32), data);
        }

        // Build the synthetic (255, 255) master index response.
        let master_blob = Self::build_master_index_blob(&hot, archive_count, private_key);
        hot.insert((255, 255), master_blob);

        info!(
            "JS5 cache ready: {} reference groups pre-loaded, remaining served on demand",
            hot.len()
        );

        Ok(Self {
            hot: Arc::new(hot),
            store: Arc::new(Mutex::new(cache)),
        })
    }

    /// Build the signed (255, 255) master index blob.
    ///
    /// Wire format (matches what the RS2 rev-647 client expects in Class234.method5721):
    ///   [comp=0:1][size:4][count:1][entries: count × 72 bytes][rsa_sig]
    ///
    /// Each 72-byte entry: [crc32:4][version:4][whirlpool_hash:64]
    ///
    /// The RSA signature covers [count:1][entries...] and is produced by:
    ///   plaintext = [magic=0x0A:1][whirlpool([count][entries]):64]
    ///   sig       = plaintext^d mod N  (sign with private key)
    ///
    /// The sig bytes are written with a leading 0x00 byte if the high bit is set,
    /// matching Java's BigInteger.toByteArray() two's-complement semantics.
    fn build_master_index_blob(
        groups: &HashMap<(u8, u32), Bytes>,
        archive_count: u8,
        private_key: &RsaKey,
    ) -> Bytes {
        // ── Step 1: build the payload ──────────────────────────────────────
        // payload = [count:1] + [entries: count × 72 bytes]
        let payload_size = 1 + archive_count as usize * 72;
        let mut payload_pkt = rs_io::Packet::new(payload_size);
        payload_pkt.p1(archive_count);
        for i in 0..archive_count as u32 {
            let entry_data = groups.get(&(255, i));
            let crc = entry_data
                .map(|d| rs_io::crc::getcrc(d, 0, d.len()))
                .unwrap_or(0);
            let whirlpool = entry_data
                .map(|d| rs_crypto::whirlpool(d))
                .unwrap_or([0u8; 64]);
            payload_pkt.p4(crc);
            payload_pkt.p4(0); // version = 0
            payload_pkt.pdata(&whirlpool, 0, whirlpool.len());
        }

        // ── Step 2: sign the payload ──────────────────────────────────────────
        // plaintext = [magic=10:1][Whirlpool(payload):64]
        let payload_bytes = &payload_pkt.data[..payload_pkt.pos];
        let hash = rs_crypto::whirlpool(payload_bytes);
        let mut plaintext_pkt = rs_io::Packet::new(65);
        plaintext_pkt.p1(10); // RSA magic
        plaintext_pkt.pdata(&hash, 0, hash.len());

        // sig = plaintext^d mod N  (RSA private-key operation)
        let plaintext_bytes = &plaintext_pkt.data[..plaintext_pkt.pos];
        let m = BigInt::from_bytes_be(Sign::Plus, plaintext_bytes);
        let mut sig = m.modpow(&private_key.d, &private_key.n).to_bytes_be().1;

        // Ensure the signature is non-negative when parsed by Java's
        // `new BigInteger(bytes)` (two's-complement): prepend 0x00 if MSB is set.
        if sig.first().map(|&b| b & 0x80 != 0).unwrap_or(false) {
            sig.insert(0, 0x00);
        }

        // ── Step 3: wrap in cache container ──────────────────────────────────
        // container = [comp=0:1][size:4][payload][sig]
        let payload_len = payload_pkt.pos;
        let total = payload_len + sig.len();
        let mut container_pkt = rs_io::Packet::new(5 + total);
        container_pkt.p1(0); // no compression
        container_pkt.p4(total as i32);
        let payload_slice = &payload_pkt.data[..payload_pkt.pos];
        container_pkt.pdata(payload_slice, 0, payload_slice.len());
        container_pkt.pdata(&sig, 0, sig.len());
        let container: Vec<u8> = container_pkt.data[..container_pkt.pos].to_vec();

        info!(
            "Master index built: {} archives, payload={} bytes, sig={} bytes",
            archive_count,
            payload_len,
            sig.len()
        );

        Bytes::from(container)
    }

    /// Fetch raw compressed bytes for a given (archive, group).
    ///
    /// Returns `None` only if the group truly doesn't exist in the cache.
    /// Hot groups (archive 255 + master index) are returned from RAM;
    /// everything else is read from disk under a short mutex lock.
    pub fn get(&self, archive: u8, group: u32) -> Option<Bytes> {
        // Fast path: pre-loaded hot groups (archive 255 reference tables).
        if let Some(b) = self.hot.get(&(archive, group)) {
            return Some(b.clone());
        }
        // Slow path: on-demand disk read.
        let mut store = self.store.lock().unwrap();
        match store.read_raw_group_bytes(archive, group) {
            Ok(Some(b)) => {
                if archive != 255 && b.len() > 2 {
                    Some(b.slice(..b.len() - 2))
                } else {
                    Some(b)
                }
            }
            _ => None,
        }
    }

    /// Compute the 27 CRC32 values for the JS5 handshake prefetch list.
    ///
    /// These are the CRC32 checksums of each archive's reference table
    /// (archive 255, groups 0-26), sent immediately after the `[0]` OK byte.
    /// The client uses them to determine which archives need downloading.
    pub fn prefetch_crcs(&self) -> [i32; 27] {
        let mut crcs = [0i32; 27];
        for i in 0..27u32 {
            if let Some(data) = self.hot.get(&(255, i)) {
                crcs[i as usize] = rs_io::crc::getcrc(data, 0, data.len());
            }
        }
        crcs
    }

    /// All 35 archive CRCs that the client sends during login.
    ///
    /// Order matches `Class130.method834` in the rev 647 client:
    /// archives 0-31, then 32 (loading_sprites), 33 (loading_screens), 36 (aJs5_100).
    ///
    /// The client computes CRC32 on the raw container bytes (the same bytes
    /// served via JS5), then decompresses separately inside Js5Index.decode().
    pub fn login_crcs(&self) -> [i32; 35] {
        const ORDER: [u32; 35] = [
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 36,
        ];
        let mut crcs = [0i32; 35];
        for (i, &aid) in ORDER.iter().enumerate() {
            if let Some(data) = self.hot.get(&(255, aid)) {
                crcs[i] = rs_io::crc::getcrc(data, 0, data.len());
            }
        }
        crcs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js5_encode_urgent() {
        // urgent=true → no 0x80 flag, no markers
        let data = vec![0u8; 100]; // byte 0 = comp_type 0
        let resp = Js5Response::new(19, 3, Bytes::from(data.clone()));
        let mut pkt = rs_io::Packet::new(resp.encoded_size());
        resp.encode(&mut pkt, true);
        let encoded = &pkt.data[..pkt.pos];
        // 3-byte header + 100 data bytes = 103 total, no markers
        assert_eq!(encoded.len(), 103);
        assert_eq!(encoded[0], 19);
        assert_eq!(encoded[1], 0);
        assert_eq!(encoded[2], 3);
        assert_eq!(&encoded[3..], data.as_slice());
    }

    #[test]
    fn test_js5_encode_large_with_markers() {
        // 600 bytes of data: one 0xFF separator is needed.
        // Output positions 0-2: archive/group prefix.
        // Data byte 509 lands at output position 512 → 0xFF inserted before it.
        let data = vec![0xABu8; 600];
        let resp = Js5Response::new(0, 0, Bytes::from(data.clone()));
        let mut pkt = rs_io::Packet::new(resp.encoded_size());
        resp.encode(&mut pkt, true);
        let encoded = &pkt.data[..pkt.pos];
        // 3 prefix + 509 data + 1 separator + 91 data = 604
        assert_eq!(encoded.len(), 604);
        // separator is at output position 512 (0-indexed)
        assert_eq!(
            encoded[512], 0xFF,
            "0xFF separator must be at output position 512"
        );
        // rest of the bytes should be 0xAB (the data), except position 0 (archive=0)
        assert_eq!(encoded[3], 0xAB); // first data byte, urgent so no 0x80
        assert_eq!(encoded[511], 0xAB); // last byte before separator
        assert_eq!(encoded[513], 0xAB); // first byte after separator
    }

    #[test]
    fn test_js5_encode_second_separator_interval() {
        // After the first separator, subsequent separators must be every 511 data
        // bytes — NOT 512. The client resets its window counter to 1 (not 0) after
        // each separator, so only 511 more bytes fill the window to 512.
        //
        // 1200 bytes of data → two separators needed.
        //   sep1: window_pos reaches 512 after 509 data bytes (3+509=512)
        //         → at encoded[512]
        //   sep2: window_pos restarts at 1, reaches 512 after 511 more bytes
        //         → at encoded[512 + 1 + 511] = encoded[1024]
        let data = vec![0xCDu8; 1200];
        let resp = Js5Response::new(0, 0, Bytes::from(data));
        let mut pkt = rs_io::Packet::new(resp.encoded_size());
        resp.encode(&mut pkt, true);
        let encoded = &pkt.data[..pkt.pos];
        // 3 + 1200 + 2 separators = 1205
        assert_eq!(encoded.len(), 1205);
        assert_eq!(encoded[512], 0xFF, "first separator at wire pos 512");
        assert_eq!(
            encoded[1024], 0xFF,
            "second separator at wire pos 1024 (not 1025)"
        );
    }

    #[test]
    fn test_js5_encode_priority_flag() {
        // Non-urgent → compression byte gets 0x80 ORed in
        let data = vec![0x01u8, 0, 0, 0, 10]; // comp_type=1 (bzip2)
        let resp = Js5Response::new(0, 0, Bytes::from(data));
        let mut pkt = rs_io::Packet::new(resp.encoded_size());
        resp.encode(&mut pkt, false);
        let encoded = &pkt.data[..pkt.pos]; // urgent=false → set 0x80
        assert_eq!(encoded[3], 0x81); // 0x01 | 0x80
    }

    #[test]
    fn test_master_index_rsa_roundtrip() {
        // Simulate what the Java client does in Class234.method5721():
        // 1. Parse the master index blob
        // 2. Extract the RSA signature
        // 3. Decrypt with public key
        // 4. Verify it's [0x0A][whirlpool_hash:64] = 65 bytes
        // 5. Verify Whirlpool of [count][entries] matches the hash

        use rs_crypto::rsa::parse_rsa_key_from_pem;

        // Test-fixture 2048-bit PKCS#8 key. The master-index signature plaintext
        // is 65 bytes ([0x0A][whirlpool:64] = 520 bits), so the signing key's
        // modulus must be larger than that — the repo's .keys/private.pem is
        // currently 512-bit, which cannot sign a 647 master index.
        const TEST_KEY_PEM: &str = "-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQCdQzsEvIb5dBQ3
XzRy+69J5EnVsgZehslAlxWbhUgbO6eF4BHXT5Y+9r8/wY7AefZzGWgzNyHwxm1w
Ws5Jn60r/ff4Njca9Eft9NWx3j124xOxFxtGqjBDTa1mXzTHbqyHNEjuK43tWWyW
vXjpcfusuv48qmGFRjbhPmqGgtQaqTrtgyoWanbgGcKAygZ4zIWR/NQv4WvD17vg
v1sVlyhUZbtshTed/OlXCrVn7kT/9ZfWAgY3y6dHHqtO4c+M4LJe/DyfLBxyHDKf
owI4CMXSfM+fEGEvkVRKhP8MjvlISS3AupagGc6oS+RPTa+IFJX0hJXG7ir4OreR
DqCsGv9xAgMBAAECggEAHADaw48mocjvyKRlXIxAvHGwTCM1SyPJNn0w1c639lF7
FTW/N43ufSvHBn1bBB8zqW8+xM/udUeAOV+SGD4cwhapT0OoMF6LaG4Ol2uxmjf5
S7GHo9+uVG3CkNYnOdQVWMYQroiG71rbIUSAAWHQpQNvPjJAx1CneRh6M5WcW1q7
qV3ipEq58U2GPKlHhlOxGOtZmsArNHOuO5r90gitLOpnoa26Bb+ojFn1tDjIVRPo
s3LSmPLv/KT9Od0SFFAwW/tYywrE5VfUIbKaTtbYA/VtoZkPnjr3SK57tsyTDhYq
CTNhaQ+bt9RUfx7h5T2CyUnfEdk2vLMSJMn52d07+QKBgQDPqlDJegXmjEGnr4CO
vUEcVV5GfERVd63Sr510qnm0V6lWekT8HV9hgRbTA/DqsHbicWVpZmgh7ss+51r3
hsTEGsw1INPScNLKpvGDT1sMGmf73W9zeeJ/osWB5pizpk5LawoBK1xKr46xtUTf
rmOjAW6kdGLSAntfm8iZ/pLxqwKBgQDB3a8nC6AM+HfQL2fS6OH8SUn0XNZi7ir+
IpVjCiT5BQmC6OuFFtgYRBtpsQUxpKXCs1/kHentglXAZyAYF5vOaSKhhhN/Tvhi
R0EMdL2LgMT8quMbQ/6liw9fjLTnd2VzCtrdOQzorERdIHm4HL4RB22uYMwA88cA
vcV2sIjvUwKBgAsSQ3haXxIpjB6KhsoVYPFHw9zyJxp/ZO8EmdpsdEyfF9Iadq8S
SdGC9lqNrTx16ACulv+FjCuy9XcHEpx7vOXPn+99Lo3bogFX4SxlAK+mgMC2z4hS
iTbQ0MLaLm31gdsXe9MQIr2AmpTy2xcpzvoTy3hNFC+qHjW8snczUVY9AoGASeWL
fzGiN296lCONyso0Yes5wmik2573Slm6Yb8bKsStxHWuWNPFWTKU1GvClBKzkIy5
L4XrhjoZH7bvd99/52UR60nlIrDiYuvQMsRI/+T8sWceHGWFqmpgWMaUqmpLhFeN
56d4cTRA9tKLMw/ZrzoKp96DxJUntPOhUkWmXLsCgYA6pJ3eC5sGDBrxxtthe/I+
15vlSF1LIVaTK1j/rUDCrfcISBUDwiX3iAY4kcntLrAFhLze2fJ7D5XY1Lahfd54
YECBv/SeBfxkckIJOj9bPbi7zXVtWXRrcj8+e6tF16WeemxvOGRY++OQ1+ktQMwn
4eEwZa3l2FKL9vv9/l6eVA==
-----END PRIVATE KEY-----";

        let keys = parse_rsa_key_from_pem(TEST_KEY_PEM).unwrap();

        // Build a minimal master index blob (2 archives for simplicity)
        let archive_count: u8 = 2;
        let mut payload_pkt = Packet::new(1 + 2 * 72);
        payload_pkt.p1(archive_count);
        for _ in 0..archive_count {
            // crc32:4 + version:4 + whirlpool:64 = 72 bytes
            payload_pkt.p4(0);
            payload_pkt.p4(0);
            payload_pkt.pdata(&[0u8; 64], 0, 64);
        }
        let payload = &payload_pkt.data[..payload_pkt.pos];
        assert_eq!(payload.len(), 1 + 2 * 72, "payload size check");

        // Sign: plaintext = [0x0A][Whirlpool(payload):64]
        let hash = rs_crypto::whirlpool(payload);
        let mut plaintext_pkt = Packet::new(65);
        plaintext_pkt.p1(0x0A);
        plaintext_pkt.pdata(&hash, 0, hash.len());
        let plaintext = &plaintext_pkt.data[..plaintext_pkt.pos];
        assert_eq!(plaintext.len(), 65);

        // Encrypt with private key: sig = plaintext^d mod n
        let m = BigInt::from_bytes_be(Sign::Plus, plaintext);
        let mut sig = m.modpow(&keys.d, &keys.n).to_bytes_be().1;

        // Prepend 0x00 if high bit set (Java BigInteger compat)
        if sig.first().map(|&b| b & 0x80 != 0).unwrap_or(false) {
            sig.insert(0, 0x00);
        }

        // Build container: [comp=0:1][size:4][payload][sig]
        let total = payload.len() + sig.len();
        let mut container_pkt = Packet::new(5 + total);
        container_pkt.p1(0);
        container_pkt.p4(total as i32);
        container_pkt.pdata(payload, 0, payload.len());
        container_pkt.pdata(&sig, 0, sig.len());
        let container = container_pkt.data[..container_pkt.pos].to_vec();

        // --- Now simulate client-side parsing ---
        // Simulate what the Java client does:
        // Skip 5 bytes header (comp + size)
        let count = container[5] as usize;
        assert_eq!(count, archive_count as usize);

        let entries_start = 6;
        let entries_end = entries_start + count * 72;
        let sig_bytes = &container[entries_end..];

        // Verify: decrypt sig with public key → should give back plaintext
        // Java: new BigInteger(sig_bytes).modPow(e, n).toByteArray()
        let m = BigInt::from_bytes_be(Sign::Plus, sig_bytes);
        let decrypted = m.modpow(&keys.e, &keys.n).to_bytes_be().1;

        // Result should be exactly 65 bytes: [0x0A][hash:64]
        assert_eq!(decrypted.len(), 65, "decrypted sig must be 65 bytes");
        assert_eq!(decrypted[0], 0x0A, "magic byte must be 0x0A");

        // Hash the payload [count][entries] and compare
        let payload_bytes = &container[5..(5 + 1 + count * 72)];
        let expected_hash = rs_crypto::whirlpool(payload_bytes);
        assert_eq!(&decrypted[1..], &expected_hash, "Whirlpool hash mismatch");
    }
}
