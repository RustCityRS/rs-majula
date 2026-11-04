#[cfg(since_647)]
use bytes::Bytes;
#[cfg(since_647)]
use num_bigint::{BigInt, Sign};
#[cfg(since_647)]
use rs_crypto::rsa::RsaKey;
#[cfg(since_647)]
use rs_crypto::whirlpool;
#[cfg(since_647)]
use rs_dat2::CacheStore;
use rs_io::Packet;
#[cfg(since_647)]
use rs_io::crc::getcrc;
#[cfg(since_647)]
use std::collections::HashMap;
#[cfg(since_647)]
use std::sync::{Arc, Mutex};
#[cfg(since_647)]
use tracing::info;

pub struct Js5Response<'a> {
    pub archive: u8,
    pub group: u16,
    pub data: &'a [u8],
}

impl Js5Response<'_> {
    #[cfg(before_647)]
    pub fn encode(&self, out: &mut Packet) {
        if self.data.is_empty() || self.data.len() > u16::MAX as usize {
            out.p1(self.archive);
            out.p2(self.group);
            out.p2(0);
            out.p1(0);
            return;
        }
        for (part, data) in self.data.chunks(500).enumerate() {
            out.p1(self.archive);
            out.p2(self.group);
            out.p2(self.data.len() as u16);
            out.p1(part as u8);
            out.pdata(data, 0, data.len());
        }
    }

    #[cfg(since_647)]
    pub fn encode(&self, out: &mut Packet, urgent: bool) {
        out.p1(self.archive);
        out.p2(self.group);
        let start = out.pos;
        let first = self.data.len().min(509);
        out.pdata(self.data, 0, first);
        if !urgent && first != 0 {
            out.data[start] |= 0x80;
        }
        for chunk in self.data[first..].chunks(511) {
            out.p1(0xFF);
            out.pdata(chunk, 0, chunk.len());
        }
    }

    #[cfg(before_647)]
    pub fn encoded_size(&self) -> usize {
        if self.data.is_empty() || self.data.len() > u16::MAX as usize {
            6
        } else {
            self.data.len() + self.data.len().div_ceil(500) * 6
        }
    }

    #[cfg(since_647)]
    pub fn encoded_size(&self) -> usize {
        size_of_val(&self.archive)
            + size_of_val(&self.group)
            + self.data.len()
            + (self.data.len() + 1) / 511
    }
}

#[cfg(since_647)]
#[derive(Clone)]
pub struct Js5Cache {
    hot: Arc<HashMap<(u8, u32), Bytes>>,
    store: Arc<Mutex<CacheStore>>,
}

#[cfg(since_647)]
impl Js5Cache {
    pub fn preload(mut cache: CacheStore, private_key: &RsaKey) -> anyhow::Result<Self> {
        let mut hot = HashMap::new();
        let archive_count = cache.group_count(255).unwrap_or(27) as u8;
        let empty = Bytes::from_static(&[0, 0, 0, 0, 4, 5, 1, 0, 0]);
        for archive in 0..archive_count {
            let data = cache
                .read_raw_group_bytes(255, archive as u32)
                .ok()
                .flatten()
                .unwrap_or_else(|| empty.clone());
            hot.insert((255, archive as u32), data);
        }
        let master = Self::build_master_index_blob(&hot, archive_count, private_key);
        hot.insert((255, 255), master);
        info!(
            "JS5 cache ready: {} reference groups pre-loaded, remaining served on demand",
            hot.len()
        );
        Ok(Self {
            hot: Arc::new(hot),
            store: Arc::new(Mutex::new(cache)),
        })
    }

    fn build_master_index_blob(
        groups: &HashMap<(u8, u32), Bytes>,
        archive_count: u8,
        private_key: &RsaKey,
    ) -> Bytes {
        let payload_len = 1 + archive_count as usize * 72;
        let mut buf = Packet::new(5 + payload_len + private_key.n.bits().div_ceil(8) as usize + 1);
        buf.p1(0);
        buf.p4(0);
        buf.p1(archive_count);
        for archive in 0..archive_count as u32 {
            let data = groups.get(&(255, archive));
            buf.p4(data.map_or(0, |data| getcrc(data, 0, data.len())));
            buf.p4(0);
            let hash = data.map_or([0; 64], |data| whirlpool(data));
            buf.pdata(&hash, 0, hash.len());
        }
        let mut signed = [10; 65];
        signed[1..].copy_from_slice(&whirlpool(&buf.data[5..buf.pos]));
        let signature = BigInt::from_bytes_be(Sign::Plus, &signed)
            .modpow(&private_key.d, &private_key.n)
            .to_signed_bytes_be();
        buf.pdata(&signature, 0, signature.len());
        buf.psize4((buf.pos - 5) as i32);
        buf.data.truncate(buf.pos);
        Bytes::from(buf.data)
    }

    pub fn get(&self, archive: u8, group: u32) -> Option<Bytes> {
        if let Some(data) = self.hot.get(&(archive, group)) {
            return Some(data.clone());
        }
        let mut store = self.store.lock().unwrap();
        let data = store.read_raw_group_bytes(archive, group).ok()??;
        if archive != 255 && data.len() > 2 {
            Some(data.slice(..data.len() - 2))
        } else {
            Some(data)
        }
    }

    pub fn prefetch_crcs(&self) -> [i32; 27] {
        let mut crcs = [0; 27];
        for (archive, crc) in crcs.iter_mut().enumerate() {
            if let Some(data) = self.hot.get(&(255, archive as u32)) {
                *crc = getcrc(data, 0, data.len());
            }
        }
        crcs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(since_647)]
    use rs_crypto::rsa::parse_rsa_key_from_pem;

    #[cfg(before_647)]
    #[test]
    fn ondemand_chunks_keep_the_file_length_and_part_order() {
        for len in [0, 1, 499, 500, 501, 1000, 1001, 65535, 65536] {
            let data: Vec<u8> = (0..len).map(|i| i as u8).collect();
            let reply = Js5Response {
                archive: 2,
                group: 0x1234,
                data: &data,
            };
            let mut buf = Packet::new(reply.encoded_size());
            reply.encode(&mut buf);
            assert_eq!(buf.pos, reply.encoded_size());
            if len == 0 || len > u16::MAX as usize {
                assert_eq!(buf.data, [2, 0x12, 0x34, 0, 0, 0]);
                continue;
            }
            let mut received = Vec::new();
            for (part, bytes) in buf.data.chunks(506).enumerate() {
                assert_eq!(&bytes[..3], &[2, 0x12, 0x34]);
                assert_eq!(u16::from_be_bytes([bytes[3], bytes[4]]) as usize, len);
                assert_eq!(bytes[5], part as u8);
                received.extend_from_slice(&bytes[6..]);
            }
            assert_eq!(received, data);
        }
    }

    #[cfg(since_647)]
    #[test]
    fn js5_chunks_match_the_client_windows() {
        for len in [0, 1, 508, 509, 510, 511, 512, 1019, 1020, 1021, 1531, 4096] {
            for urgent in [false, true] {
                let reply = Js5Response {
                    archive: 19,
                    group: 0xABCD,
                    data: &vec![1; len],
                };
                let mut buf = Packet::new(reply.encoded_size() + 1);
                buf.p1(42);
                reply.encode(&mut buf, urgent);
                let bytes = &buf.data[1..buf.pos];
                assert_eq!(bytes.len(), reply.encoded_size());
                assert_eq!(&bytes[..3], &[19, 0xAB, 0xCD]);
                let mut data = Vec::new();
                for (pos, &byte) in bytes.iter().enumerate().skip(3) {
                    if pos % 512 == 0 {
                        assert_eq!(byte, 0xFF);
                    } else {
                        data.push(byte);
                    }
                }
                let mut expected = vec![1; len];
                if !urgent && len > 0 {
                    expected[0] |= 0x80;
                }
                assert_eq!(data, expected);
            }
        }
    }

    #[cfg(since_647)]
    #[test]
    fn master_index_signature_matches_the_served_reference_tables() {
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
        let reference = Bytes::from_static(&[0, 0, 0, 0, 4, 5, 1, 0, 0]);
        let groups = HashMap::from([((255, 0), reference.clone()), ((255, 1), reference.clone())]);
        let container = Js5Cache::build_master_index_blob(&groups, 2, &keys);
        assert_eq!(container[0], 0);
        assert_eq!(
            u32::from_be_bytes(container[1..5].try_into().unwrap()) as usize,
            container.len() - 5
        );
        assert_eq!(container[5], 2);
        for entry in container[6..150].as_chunks::<72>().0 {
            assert_eq!(
                i32::from_be_bytes(entry[..4].try_into().unwrap()),
                getcrc(&reference, 0, reference.len())
            );
            assert_eq!(&entry[4..8], &[0; 4]);
            assert_eq!(&entry[8..], &whirlpool(&reference));
        }
        let signed = BigInt::from_signed_bytes_be(&container[150..])
            .modpow(&keys.e, &keys.n)
            .to_signed_bytes_be();
        assert_eq!(signed.len(), 65);
        assert_eq!(signed[0], 10);
        assert_eq!(&signed[1..], &whirlpool(&container[5..150]));
    }
}
