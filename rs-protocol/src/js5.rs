#[cfg(since_647)]
use rs_crypto::rsa::RsaKey;
use rs_io::Packet;
#[cfg(since_647)]
use rs_io::crc::getcrc;
#[cfg(since_647)]
use rs_io::js5::Js5Store;
#[cfg(since_647)]
use rs_io::js5::compression::Container;
#[cfg(since_647)]
use rs_io::js5::reference::encode_master_index;
#[cfg(since_647)]
use std::borrow::Cow;
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
    hot: Arc<HashMap<(u8, u32), Vec<u8>>>,
    store: Arc<Mutex<Js5Store>>,
}

#[cfg(since_647)]
impl Js5Cache {
    pub fn preload(mut cache: Js5Store, private_key: &RsaKey) -> anyhow::Result<Self> {
        let mut hot = HashMap::new();
        let archive_count = cache.group_count(255).unwrap_or(27) as u8;
        for archive in 0..archive_count {
            let data = cache
                .read_raw_group_bytes(255, archive as u32)
                .ok()
                .flatten()
                .unwrap_or_else(|| vec![0, 0, 0, 0, 4, 5, 1, 0, 0]);
            hot.insert((255, archive as u32), data);
        }
        let master = encode_master_index(
            (0..archive_count as u32).map(|archive| (hot[&(255, archive)].as_ref(), 0)),
            private_key,
        );
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

    pub fn get(&self, archive: u8, group: u32) -> Option<Cow<'_, [u8]>> {
        if let Some(data) = self.hot.get(&(archive, group)) {
            return Some(Cow::Borrowed(data));
        }
        let mut store = self.store.lock().unwrap();
        let mut data = store.read_raw_group_bytes(archive, group).ok()??;
        let len = Container::parse(&data).ok()?.data.len();
        data.truncate(len);
        Some(Cow::Owned(data))
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
}
