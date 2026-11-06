#[cfg(since_647)]
use rs_crypto::rsa::RsaKey;
use rs_io::Packet;
#[cfg(since_647)]
use rs_io::crc::getcrc;
#[cfg(since_647)]
use rs_io::js5::compression::Container;
#[cfg(since_647)]
use rs_io::js5::reference::encode_master_index;
#[cfg(since_647)]
use std::collections::HashMap;
#[cfg(since_647)]
use std::sync::Arc;
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
    groups: Arc<HashMap<(u8, u32), Vec<u8>>>,
}

#[cfg(since_647)]
impl Js5Cache {
    pub fn new(
        mut groups: HashMap<(u8, u32), Vec<u8>>,
        private_key: &RsaKey,
    ) -> anyhow::Result<Self> {
        let archive_count = groups
            .keys()
            .filter(|(archive, _)| *archive == 255)
            .map(|(_, group)| group + 1)
            .max()
            .unwrap_or(0);
        for archive in 0..archive_count {
            groups
                .entry((255, archive))
                .or_insert_with(|| vec![0, 0, 0, 0, 4, 5, 1, 0, 0]);
        }
        for data in groups.values_mut() {
            let len = Container::parse(data)?.data.len();
            data.truncate(len);
        }
        let master = encode_master_index(
            (0..archive_count).map(|archive| (groups[&(255, archive)].as_ref(), 0)),
            private_key,
        );
        groups.insert((255, 255), master);
        info!("JS5 cache ready: {} groups in memory", groups.len());
        Ok(Self {
            groups: Arc::new(groups),
        })
    }

    pub fn get(&self, archive: u8, group: u32) -> Option<&[u8]> {
        self.groups.get(&(archive, group)).map(Vec::as_slice)
    }

    pub fn prefetch_crcs(&self) -> [i32; 27] {
        let mut crcs = [0; 27];
        for (archive, crc) in crcs.iter_mut().enumerate() {
            if let Some(data) = self.groups.get(&(255, archive as u32)) {
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
    use rs_crypto::rsa::load_rsa_key;
    #[cfg(since_647)]
    use rs_io::js5::compression::{CompressionType, compress};

    #[cfg(since_647)]
    #[test]
    fn memory_cache_serves_containers_without_versions_and_signs_the_complete_index() {
        let key = load_rsa_key(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../.keys/private-647.pem"
        ))
        .unwrap();
        let reference = vec![0, 0, 0, 0, 4, 5, 1, 0, 0];
        let container = compress(&[11, 22, 33], CompressionType::Gzip).unwrap();
        let mut versioned = container.clone();
        versioned.extend_from_slice(&321u16.to_be_bytes());
        let cache = Js5Cache::new(
            HashMap::from([
                ((255, 0), reference.clone()),
                ((255, 2), reference.clone()),
                ((2, 7), versioned),
            ]),
            &key,
        )
        .unwrap();
        assert_eq!(cache.get(2, 7), Some(container.as_slice()));
        assert_eq!(cache.get(255, 1), Some(reference.as_slice()));
        assert!(cache.get(2, 8).is_none());
        let master = cache.get(255, 255).unwrap();
        assert_eq!(Container::parse(master).unwrap().data.len(), master.len());
        assert_eq!(master[5], 3);
        assert_eq!(
            &cache.prefetch_crcs()[..3],
            &[getcrc(&reference, 0, reference.len()); 3]
        );
        let shared = cache.clone();
        assert!(std::ptr::eq(
            cache.get(2, 7).unwrap(),
            shared.get(2, 7).unwrap()
        ));
    }

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
