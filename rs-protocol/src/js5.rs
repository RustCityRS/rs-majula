#[cfg(since_647)]
use Prefetch::{Archive, Group};
#[cfg(since_530)]
use rs_crypto::rsa::RsaKey;
use rs_io::Packet;
#[cfg(all(since_530, before_647))]
use rs_io::crc::getcrc;
#[cfg(since_530)]
use rs_io::js5::compression::Container;
#[cfg(since_530)]
use rs_io::js5::index::Js5Index;
#[cfg(since_647)]
use rs_io::js5::index::encode_master_index;
#[cfg(since_647)]
use rs_io::stringtools::hash32;
#[cfg(since_530)]
use rs_ver::archive::REFERENCE_TABLES;
#[cfg(since_647)]
use rs_ver::archive::{
    BILLBOARDS, BINARY, CLIENTSCRIPT, CONFIG, CONFIG_ENUM, CONFIG_LOC, CONFIG_NPC, CONFIG_OBJ,
    CONFIG_SEQ, CONFIG_SPOTANIM, CONFIG_VARPBIT, DEFAULTS, DLLS, FONTMETRICS, INTERFACES,
    MATERIALS, PARTICLES, QUICKCHAT, QUICKCHAT_GLOBAL, SHADERS, WORLDMAP,
};
#[cfg(since_530)]
use std::collections::{HashMap, HashSet};
#[cfg(since_530)]
use std::ops::Range;
#[cfg(since_530)]
use std::sync::Arc;
#[cfg(since_530)]
use tracing::info;

#[cfg(before_742)]
pub type Js5Group = u16;
#[cfg(since_742)]
pub type Js5Group = u32;

#[cfg(since_530)]
const BLOCK: usize = 512;
#[cfg(since_530)]
const FIRST_BLOCK: usize = BLOCK - size_of::<u8>() - size_of::<Js5Group>();

pub struct Js5Response<'a> {
    pub archive: u8,
    pub group: Js5Group,
    pub data: &'a [u8],
}

impl Js5Response<'_> {
    #[cfg(before_530)]
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

    #[cfg(since_530)]
    pub fn encode(&self, out: &mut Packet, urgent: bool) {
        out.p1(self.archive);
        #[cfg(before_742)]
        out.p2(self.group);
        #[cfg(since_742)]
        out.p4(self.group as i32);
        let start = out.pos;
        let first = self.data.len().min(FIRST_BLOCK);
        out.pdata(self.data, 0, first);
        if !urgent && first != 0 {
            out.data[start] |= 0x80;
        }
        for chunk in self.data[first..].chunks(BLOCK - 1) {
            out.p1(0xFF);
            out.pdata(chunk, 0, chunk.len());
        }
    }

    #[cfg(before_530)]
    pub fn encoded_size(&self) -> usize {
        if self.data.is_empty() || self.data.len() > u16::MAX as usize {
            6
        } else {
            self.data.len() + self.data.len().div_ceil(500) * 6
        }
    }

    #[cfg(since_530)]
    pub fn encoded_size(&self) -> usize {
        size_of_val(&self.archive)
            + size_of_val(&self.group)
            + self.data.len()
            + (self.data.len() + BLOCK - 2 - FIRST_BLOCK) / (BLOCK - 1)
    }
}

#[cfg(since_647)]
enum Prefetch {
    Archive(u8),
    Group(u8, &'static str),
}

#[cfg(all(since_647, before_742))]
const PREFETCH_COUNT: usize = 27;
#[cfg(since_742)]
const PREFETCH_COUNT: usize = 26;

#[cfg(since_647)]
const PREFETCHES: [Prefetch; PREFETCH_COUNT] = [
    Archive(DEFAULTS),
    Group(DLLS, "windows/x86/jaclib.dll"),
    Group(DLLS, "windows/x86/jaggl.dll"),
    Group(DLLS, "windows/x86/jagdx.dll"),
    #[cfg(before_742)]
    Group(DLLS, "windows/x86/jagmisc.dll"),
    Group(DLLS, "windows/x86/sw3d.dll"),
    Group(DLLS, "windows/x86/hw3d.dll"),
    Group(DLLS, "windows/x86/jagtheora.dll"),
    Archive(SHADERS),
    Archive(MATERIALS),
    Archive(CONFIG),
    Archive(CONFIG_LOC),
    Archive(CONFIG_ENUM),
    Archive(CONFIG_NPC),
    Archive(CONFIG_OBJ),
    Archive(CONFIG_SEQ),
    Archive(CONFIG_SPOTANIM),
    Archive(CONFIG_VARPBIT),
    Archive(QUICKCHAT),
    Archive(QUICKCHAT_GLOBAL),
    Archive(PARTICLES),
    Archive(BILLBOARDS),
    Group(BINARY, "huffman"),
    Archive(INTERFACES),
    Archive(CLIENTSCRIPT),
    Archive(FONTMETRICS),
    Group(WORLDMAP, "details"),
];

#[cfg(since_530)]
#[derive(Clone)]
pub struct Js5Cache {
    groups: Arc<HashMap<(u8, u32), Range<usize>>>,
    data: Arc<Vec<u8>>,
    absent: Arc<HashSet<(u8, u32)>>,
    #[cfg(since_647)]
    prefetches: [i32; PREFETCH_COUNT],
}

#[cfg(since_530)]
impl Js5Cache {
    pub fn new(
        mut groups: HashMap<(u8, u32), Vec<u8>>,
        tables: &HashMap<u8, Js5Index>,
        #[cfg_attr(all(since_530, before_647), allow(unused_variables))] private_key: &RsaKey,
    ) -> anyhow::Result<Self> {
        let archive_count = groups
            .keys()
            .filter(|(archive, _)| *archive == REFERENCE_TABLES)
            .map(|(_, group)| group + 1)
            .max()
            .unwrap_or(0);
        for archive in 0..archive_count {
            groups
                .entry((REFERENCE_TABLES, archive))
                .or_insert_with(|| vec![0, 0, 0, 0, 4, 5, 1, 0, 0]);
        }
        for data in groups.values_mut() {
            let len = Container::parse(data)?.data.len();
            data.truncate(len);
        }
        #[cfg(all(since_530, before_647))]
        let master = encode_master_index((0..archive_count).map(|archive| {
            let version = tables
                .get(&(archive as u8))
                .filter(|table| table.format >= 6)
                .map_or(0, |table| table.index_version);
            (groups[&(REFERENCE_TABLES, archive)].as_ref(), version)
        }));
        #[cfg(since_647)]
        let master = encode_master_index(
            (0..archive_count).map(|archive| (groups[&(REFERENCE_TABLES, archive)].as_ref(), 0)),
            private_key,
        );
        groups.insert((REFERENCE_TABLES, REFERENCE_TABLES as u32), master);
        let mut data = Vec::with_capacity(groups.values().map(Vec::len).sum());
        let groups: HashMap<_, _> = groups
            .into_iter()
            .map(|(id, group)| {
                let start = data.len();
                data.extend_from_slice(&group);
                (id, start..data.len())
            })
            .collect();
        let absent = tables
            .iter()
            .flat_map(|(&archive, table)| table.groups.iter().map(move |group| (archive, group.id)))
            .filter(|id| !groups.contains_key(id))
            .collect();
        #[cfg(since_647)]
        let mut archive_bytes = [0; REFERENCE_TABLES as usize];
        #[cfg(since_647)]
        for (&(archive, _), range) in &groups {
            if archive != REFERENCE_TABLES {
                archive_bytes[archive as usize] += range.len();
            }
        }
        #[cfg(since_647)]
        let len = |archive: u8, group: u32| groups.get(&(archive, group)).map_or(0, Range::len);
        #[cfg(since_647)]
        let prefetches = PREFETCHES.map(|prefetch| {
            let size = match prefetch {
                Archive(archive) => {
                    len(REFERENCE_TABLES, archive as u32) + archive_bytes[archive as usize]
                }
                Group(archive, name) => {
                    let hash = hash32(name);
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
            absent: Arc::new(absent),
            #[cfg(since_647)]
            prefetches,
        })
    }

    pub fn get(&self, archive: u8, group: impl Into<u32>) -> Option<&[u8]> {
        self.groups
            .get(&(archive, group.into()))
            .map(|range| &self.data[range.clone()])
    }

    /// Whether `group` is listed in its archive's reference table without data,
    /// as in the original cache.
    pub fn is_absent(&self, archive: u8, group: impl Into<u32>) -> bool {
        self.absent.contains(&(archive, group.into()))
    }

    #[cfg(since_647)]
    pub fn prefetches(&self) -> [i32; PREFETCH_COUNT] {
        self.prefetches
    }
}

#[cfg(all(since_530, before_647))]
fn encode_master_index<'a>(archives: impl ExactSizeIterator<Item = (&'a [u8], u32)>) -> Vec<u8> {
    let len = archives.len() << 3;
    let mut out = Packet::new(5 + len);
    out.p1(0);
    out.p4(len as i32);
    for (data, version) in archives {
        out.p4(getcrc(data, 0, data.len()));
        out.p4(version as i32);
    }
    out.data
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(since_530)]
    use rs_crypto::rsa::load_rsa_key;
    #[cfg(since_530)]
    use rs_io::js5::compression::{CompressionType, compress};
    #[cfg(since_530)]
    use rs_io::js5::index::GroupInfo;
    #[cfg(since_647)]
    use rs_ver::archive::{CONFIG_GROUP_FLU, CONFIG_GROUP_HUNT};
    #[cfg(since_530)]
    use std::ptr;

    #[cfg(all(since_530, before_647))]
    #[test]
    fn memory_cache_serves_containers_without_versions_and_an_unsigned_index() {
        let key = load_rsa_key(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../.keys/classic/private.pem"
        ))
        .unwrap();
        let reference = vec![0, 0, 0, 0, 4, 5, 1, 0, 0];
        let container = compress(&[11, 22, 33], CompressionType::Gzip).unwrap();
        let mut versioned = container.clone();
        versioned.extend_from_slice(&321u16.to_be_bytes());
        let group = |id| GroupInfo {
            id,
            name_hash: 0,
            crc: 0,
            whirlpool: None,
            version: 0,
            size: 0,
            file_count: 1,
            files: Vec::new(),
        };
        let table = |archive_id, format, index_version, groups| Js5Index {
            archive_id,
            format,
            index_version,
            named: false,
            has_group_whirlpools: false,
            groups,
        };
        let cache = Js5Cache::new(
            HashMap::from([
                ((REFERENCE_TABLES, 0), reference.clone()),
                ((REFERENCE_TABLES, 2), reference.clone()),
                ((2, 7), versioned),
            ]),
            &HashMap::from([
                (0, table(0, 5, 9, Vec::new())),
                (2, table(2, 6, 77, vec![group(7), group(8)])),
            ]),
            &key,
        )
        .unwrap();
        assert_eq!(cache.get(2, 7u32), Some(container.as_slice()));
        assert!(cache.get(2, 8u32).is_none());
        assert!(cache.is_absent(2, 8u32));
        assert!(!cache.is_absent(2, 7u32));
        assert!(!cache.is_absent(2, 9u32));
        assert_eq!(
            cache.get(REFERENCE_TABLES, 1u32),
            Some(reference.as_slice())
        );
        let crc = getcrc(&reference, 0, reference.len()).to_be_bytes();
        let mut expected = vec![0, 0, 0, 0, 24];
        for version in [0, 0, 77] {
            expected.extend_from_slice(&crc);
            expected.extend_from_slice(&i32::to_be_bytes(version));
        }
        assert_eq!(
            cache.get(REFERENCE_TABLES, REFERENCE_TABLES as u32),
            Some(expected.as_slice())
        );
        let shared = cache.clone();
        assert!(ptr::eq(
            cache.get(2, 7u32).unwrap(),
            shared.get(2, 7u32).unwrap()
        ));
    }

    #[cfg(since_647)]
    #[test]
    fn memory_cache_serves_containers_without_versions_and_signs_the_complete_index() {
        let key = load_rsa_key(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../.keys/js5/private.pem"
        ))
        .unwrap();
        let reference = vec![0, 0, 0, 0, 4, 5, 1, 0, 0];
        let container = compress(&[11, 22, 33], CompressionType::Gzip).unwrap();
        let mut versioned = container.clone();
        versioned.extend_from_slice(&321u16.to_be_bytes());
        let cache = Js5Cache::new(
            HashMap::from([
                ((REFERENCE_TABLES, 0), reference.clone()),
                ((REFERENCE_TABLES, 2), reference.clone()),
                ((2, 7), versioned),
            ]),
            &HashMap::new(),
            &key,
        )
        .unwrap();
        assert_eq!(cache.get(2, 7u32), Some(container.as_slice()));
        assert_eq!(
            cache.get(REFERENCE_TABLES, 1u32),
            Some(reference.as_slice())
        );
        assert!(cache.get(2, 8u32).is_none());
        let master = cache
            .get(REFERENCE_TABLES, REFERENCE_TABLES as u32)
            .unwrap();
        assert_eq!(Container::parse(master).unwrap().data.len(), master.len());
        assert_eq!(master[5], 3);
        let shared = cache.clone();
        assert!(ptr::eq(
            cache.get(2, 7u32).unwrap(),
            shared.get(2, 7u32).unwrap()
        ));
    }

    #[cfg(since_647)]
    #[test]
    fn prefetches_weigh_each_loading_requirement_by_its_download_bytes() {
        let key = load_rsa_key(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../.keys/js5/private.pem"
        ))
        .unwrap();
        let table = vec![0, 0, 0, 0, 4, 5, 1, 0, 0];
        let first = compress(&[1; 600], CompressionType::Gzip).unwrap();
        let second = compress(&[2; 40], CompressionType::Gzip).unwrap();
        let huffman = compress(&[3; 20], CompressionType::None).unwrap();
        let mut versioned = huffman.clone();
        versioned.extend_from_slice(&[0, 7]);
        let binary = Js5Index {
            archive_id: BINARY,
            format: 5,
            index_version: 0,
            named: true,
            has_group_whirlpools: false,
            groups: vec![GroupInfo {
                id: 4,
                name_hash: hash32("huffman"),
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
                ((REFERENCE_TABLES, CONFIG as u32), table.clone()),
                ((REFERENCE_TABLES, BINARY as u32), table.clone()),
                ((CONFIG, CONFIG_GROUP_FLU), first.clone()),
                ((CONFIG, CONFIG_GROUP_HUNT), second.clone()),
                ((BINARY, 4), versioned),
            ]),
            &HashMap::from([(BINARY, binary)]),
            &key,
        )
        .unwrap();
        let prefetches = cache.prefetches();
        let config = PREFETCHES
            .iter()
            .position(|p| matches!(p, Archive(CONFIG)))
            .unwrap();
        let huffman_at = PREFETCHES
            .iter()
            .position(|p| matches!(p, Group(BINARY, "huffman")))
            .unwrap();
        assert_eq!(
            prefetches[config] as usize,
            table.len() + first.len() + second.len()
        );
        assert_eq!(prefetches[huffman_at] as usize, huffman.len());
        assert_eq!(prefetches[0], 0);
        assert_eq!(prefetches[PREFETCH_COUNT - 1], 0);
    }

    #[cfg(before_530)]
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

    #[cfg(since_530)]
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
                let header = BLOCK - FIRST_BLOCK;
                assert_eq!(bytes[0], 19);
                assert_eq!(&bytes[1..header], &reply.group.to_be_bytes());
                let mut data = Vec::new();
                for (pos, &byte) in bytes.iter().enumerate().skip(header) {
                    if pos % BLOCK == 0 {
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
