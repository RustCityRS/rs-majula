#[cfg(since_647)]
use Prefetch::{Archive, Group};
#[cfg(since_647)]
use rs_crypto::rsa::RsaKey;
use rs_io::Packet;
#[cfg(since_647)]
use rs_io::js5::compression::Container;
#[cfg(since_647)]
use rs_io::js5::java_string_hash;
#[cfg(since_647)]
use rs_io::js5::reference::{ReferenceTable, encode_master_index};
#[cfg(since_647)]
use std::collections::HashMap;
#[cfg(since_647)]
use std::ops::Range;
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
enum Prefetch {
    Archive(u8),
    Group(u8, &'static str),
}

#[cfg(since_647)]
const PREFETCHES: [Prefetch; 27] = [
    Archive(28),
    Group(30, "windows/x86/jaclib.dll"),
    Group(30, "windows/x86/jaggl.dll"),
    Group(30, "windows/x86/jagdx.dll"),
    Group(30, "windows/x86/jagmisc.dll"),
    Group(30, "windows/x86/sw3d.dll"),
    Group(30, "windows/x86/hw3d.dll"),
    Group(30, "windows/x86/jagtheora.dll"),
    Archive(31),
    Archive(26),
    Archive(2),
    Archive(16),
    Archive(17),
    Archive(18),
    Archive(19),
    Archive(20),
    Archive(21),
    Archive(22),
    Archive(24),
    Archive(25),
    Archive(27),
    Archive(29),
    Group(10, "huffman"),
    Archive(3),
    Archive(12),
    Archive(13),
    Group(23, "details"),
];

#[cfg(since_647)]
#[derive(Clone)]
pub struct Js5Cache {
    groups: Arc<HashMap<(u8, u32), Range<usize>>>,
    data: Arc<Vec<u8>>,
    prefetches: [i32; 27],
}

#[cfg(since_647)]
impl Js5Cache {
    pub fn new(
        mut groups: HashMap<(u8, u32), Vec<u8>>,
        tables: &HashMap<u8, ReferenceTable>,
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
        let mut data = Vec::with_capacity(groups.values().map(Vec::len).sum());
        let groups: HashMap<_, _> = groups
            .into_iter()
            .map(|(id, group)| {
                let start = data.len();
                data.extend_from_slice(&group);
                (id, start..data.len())
            })
            .collect();
        let mut archive_bytes = [0; 255];
        for (&(archive, _), range) in &groups {
            if archive != 255 {
                archive_bytes[archive as usize] += range.len();
            }
        }
        let len = |archive: u8, group: u32| groups.get(&(archive, group)).map_or(0, Range::len);
        let prefetches = PREFETCHES.map(|prefetch| {
            let size = match prefetch {
                Archive(archive) => len(255, archive as u32) + archive_bytes[archive as usize],
                Group(archive, name) => {
                    let hash = java_string_hash(name);
                    tables
                        .get(&archive)
                        .and_then(|table| table.groups.iter().find(|group| group.name_hash == hash))
                        .map_or(0, |group| len(archive, group.id))
                }
            };
            size as i32
        });
        info!("JS5 cache ready: {} groups in memory", groups.len());
        Ok(Self {
            groups: Arc::new(groups),
            data: Arc::new(data),
            prefetches,
        })
    }

    pub fn get(&self, archive: u8, group: u32) -> Option<&[u8]> {
        self.groups
            .get(&(archive, group))
            .map(|range| &self.data[range.clone()])
    }

    pub fn prefetches(&self) -> [i32; 27] {
        self.prefetches
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
    use rs_io::js5::reference::GroupInfo;
    #[cfg(since_647)]
    use std::ptr;

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
            &HashMap::new(),
            &key,
        )
        .unwrap();
        assert_eq!(cache.get(2, 7), Some(container.as_slice()));
        assert_eq!(cache.get(255, 1), Some(reference.as_slice()));
        assert!(cache.get(2, 8).is_none());
        let master = cache.get(255, 255).unwrap();
        assert_eq!(Container::parse(master).unwrap().data.len(), master.len());
        assert_eq!(master[5], 3);
        let shared = cache.clone();
        assert!(ptr::eq(cache.get(2, 7).unwrap(), shared.get(2, 7).unwrap()));
    }

    #[cfg(since_647)]
    #[test]
    fn prefetches_weigh_each_loading_requirement_by_its_download_bytes() {
        let key = load_rsa_key(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../.keys/private-647.pem"
        ))
        .unwrap();
        let table = vec![0, 0, 0, 0, 4, 5, 1, 0, 0];
        let first = compress(&[1; 600], CompressionType::Gzip).unwrap();
        let second = compress(&[2; 40], CompressionType::Gzip).unwrap();
        let huffman = compress(&[3; 20], CompressionType::None).unwrap();
        let mut versioned = huffman.clone();
        versioned.extend_from_slice(&[0, 7]);
        let binary = ReferenceTable {
            archive_id: 10,
            format: 5,
            version: 0,
            named: true,
            whirlpooled: false,
            groups: vec![GroupInfo {
                id: 4,
                name_hash: java_string_hash("huffman"),
                crc: 0,
                whirlpool: None,
                version: 7,
                size: 0,
                file_count: 1,
                files: Vec::new(),
            }],
        };
        let cache = Js5Cache::new(
            HashMap::from([
                ((255, 2), table.clone()),
                ((255, 10), table.clone()),
                ((2, 0), first.clone()),
                ((2, 1), second.clone()),
                ((10, 4), versioned),
            ]),
            &HashMap::from([(10, binary)]),
            &key,
        )
        .unwrap();
        let prefetches = cache.prefetches();
        assert_eq!(
            prefetches[10] as usize,
            table.len() + first.len() + second.len()
        );
        assert_eq!(prefetches[22] as usize, huffman.len());
        assert_eq!(prefetches[0], 0);
        assert_eq!(prefetches[26], 0);
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
