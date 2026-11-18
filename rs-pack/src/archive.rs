#[cfg(since_530)]
use crate::cache::js5::PackedCache;
use anyhow::Result;
#[cfg(since_530)]
use anyhow::{Context, anyhow, bail, ensure};
#[cfg(since_530)]
use rayon::prelude::*;
#[cfg(since_530)]
use rs_crypto::whirlpool;
#[cfg(since_530)]
use rs_io::crc;
#[cfg(before_530)]
use rs_io::jag::JagCompression;
#[cfg(all(since_244, before_530))]
use rs_io::js5::Js5LocalCache;
#[cfg(since_530)]
use rs_io::js5::compression::{CompressionType, Container, decompress};
#[cfg(since_530)]
use rs_io::js5::index::{GroupInfo, encode_index};
#[cfg(since_530)]
use std::collections::HashMap;
#[cfg(any(rev = "225", since_530))]
use std::fs;
use std::path::Path;
#[cfg(rev = "225")]
use std::path::PathBuf;
#[cfg(since_530)]
use tracing::{info, warn};

#[cfg(since_244)]
pub use rs_ver::archive::*;

#[cfg(before_530)]
#[derive(Clone, Copy)]
pub enum JagArchive {
    Models = 0,
    Title = 1,
    Config = 2,
    Interface = 3,
    Media = 4,
    #[cfg(since_244)]
    VersionList = 5,
    Textures = 6,
    Wordenc = 7,
    Sounds = 8,
}

#[cfg(before_530)]
pub(crate) fn config_jag_name(kind: &str) -> &str {
    if kind == "varpbit" { "varbit" } else { kind }
}

#[cfg(before_530)]
pub const JAG_ARCHIVES: &[JagArchive] = &[
    JagArchive::Title,
    JagArchive::Config,
    JagArchive::Interface,
    JagArchive::Media,
    #[cfg(rev = "225")]
    JagArchive::Models,
    #[cfg(since_244)]
    JagArchive::VersionList,
    JagArchive::Textures,
    JagArchive::Wordenc,
    JagArchive::Sounds,
];

#[cfg(before_530)]
impl JagArchive {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Models => "models",
            Self::Title => "title",
            Self::Config => "config",
            Self::Interface => "interface",
            Self::Media => "media",
            #[cfg(since_244)]
            Self::VersionList => "versionlist",
            Self::Textures => "textures",
            Self::Wordenc => "wordenc",
            Self::Sounds => "sounds",
        }
    }

    pub const fn id(self) -> Option<u32> {
        #[cfg(rev = "225")]
        {
            None
        }
        #[cfg(since_244)]
        {
            match self {
                Self::Models => None,
                _ => Some(self as u32),
            }
        }
    }

    pub const fn expected_crc(self) -> Option<i32> {
        match self {
            Self::Models => {
                #[cfg(rev = "225")]
                {
                    EXPECTED_CRCS[4]
                }
                #[cfg(since_244)]
                {
                    None
                }
            }
            _ => EXPECTED_CRCS[self as usize - 1],
        }
    }

    pub const fn compression(self) -> JagCompression {
        match self {
            Self::Interface => JagCompression::WholeArchive,
            #[cfg(since_244)]
            Self::VersionList => JagCompression::WholeArchive,
            _ => JagCompression::PerFile,
        }
    }
}

#[cfg(before_530)]
pub(crate) struct JagSource {
    #[cfg(rev = "225")]
    dir: PathBuf,
    #[cfg(since_244)]
    cache: Js5LocalCache,
}

#[cfg(before_530)]
impl JagSource {
    pub(crate) fn open(expected_dir: &Path) -> Result<Self> {
        Ok(Self {
            #[cfg(rev = "225")]
            dir: expected_dir.to_path_buf(),
            #[cfg(since_244)]
            cache: Js5LocalCache::open(expected_dir)?,
        })
    }

    pub(crate) fn archive(&mut self, kind: JagArchive) -> Option<Vec<u8>> {
        #[cfg(rev = "225")]
        {
            let name = kind.name();
            fs::read(self.dir.join("archives").join(name))
                .or_else(|_| fs::read(self.dir.join(name)))
                .ok()
        }
        #[cfg(since_244)]
        {
            self.cache
                .read_raw_group_bytes(JAG, kind.id()?)
                .ok()
                .flatten()
                .map(|b| b.to_vec())
        }
    }

    #[cfg(since_244)]
    pub(crate) fn cache(&mut self) -> &mut Js5LocalCache {
        &mut self.cache
    }
}

#[cfg(rev = "225")]
pub const EXPECTED_CRCS: &[Option<i32>] = &[
    Some(-430779560),
    Some(511217062),
    Some(1614084464),
    Some(-343404987),
    Some(-2000991154),
    Some(1703545114),
    Some(1570981179),
    Some(-1532605973),
];

#[cfg(rev = "244")]
pub const EXPECTED_CRCS: &[Option<i32>] = &[
    Some(126707642),
    Some(1573679574),
    Some(2074207176),
    Some(-151945349),
    Some(-390182005),
    Some(245278618),
    Some(-87627495),
    Some(-855112082),
];

#[cfg(rev = "245.2")]
pub const EXPECTED_CRCS: &[Option<i32>] = &[
    Some(126707642),
    Some(219495412),
    Some(1539972921),
    Some(353992155),
    Some(-1979342254),
    Some(-1885459577),
    Some(-87627495),
    Some(-1625923170),
];

#[cfg(rev = "254")]
pub const EXPECTED_CRCS: &[Option<i32>] = &[
    Some(1187152444),
    Some(1524313696),
    Some(531876099),
    Some(-374324307),
    Some(-128580638),
    Some(-1826159457),
    Some(1385372455),
    Some(392114586),
];

#[cfg(rev = "274")]
pub const EXPECTED_CRCS: &[Option<i32>] = &[
    Some(410306098),
    Some(-433051697),
    Some(2135735991),
    Some(1861649167),
    Some(-322040827),
    Some(915347346),
    Some(1386621111),
    Some(-759577225),
];

#[cfg(rev = "289")]
pub const EXPECTED_CRCS: &[Option<i32>] = &[
    Some(-564448443),
    Some(1760335781),
    Some(123011849),
    Some(-1710005394),
    Some(-868295148),
    Some(915347346),
    Some(455063218),
    Some(-1491954823),
];

#[cfg(rev = "530")]
pub const EXPECTED_CRCS: &[Option<i32>] = &[
    Some(0x28f9dd35_u32 as i32), // [ 0] anims
    Some(0xcd698fd9_u32 as i32), // [ 1] bases
    Some(0xa73b1e66_u32 as i32), // [ 2] config
    Some(0x0f3350d1_u32 as i32), // [ 3] interfaces
    Some(0x0dfb0315_u32 as i32), // [ 4] synths
    Some(0x57d08641_u32 as i32), // [ 5] maps
    Some(0x54c0f2ae_u32 as i32), // [ 6] music
    Some(0x7b7d3f12_u32 as i32), // [ 7] models
    Some(0xa939b8c3_u32 as i32), // [ 8] sprites
    Some(0x7bfab336_u32 as i32), // [ 9] textures
    Some(0x2976276a_u32 as i32), // [10] binary
    Some(0x9a9dd384_u32 as i32), // [11] jingles
    Some(0x63a46915_u32 as i32), // [12] clientscripts
    Some(0x8d1bfe8d_u32 as i32), // [13] fontmetrics
    Some(0xac7a0172_u32 as i32), // [14] vorbis
    Some(0x6c7a6953_u32 as i32), // [15] instruments
    Some(0x5ec735b8_u32 as i32), // [16] config_loc
    Some(0x873e745f_u32 as i32), // [17] config_enum
    Some(0x96d02faa_u32 as i32), // [18] config_npc
    Some(0x7d462536_u32 as i32), // [19] config_obj
    Some(0xe84148ba_u32 as i32), // [20] config_seq
    Some(0x4e48b2b9_u32 as i32), // [21] config_spot
    Some(0xb1fa9938_u32 as i32), // [22] config_varpbit
    Some(0x1ccb93db_u32 as i32), // [23] worldmap
    Some(0x778bda72_u32 as i32), // [24] quickchat
    Some(0x53cce6cc_u32 as i32), // [25] quickchat_global
    Some(0xea0d671f_u32 as i32), // [26] materials
    Some(0x2457425c_u32 as i32), // [27] config_particle
];

#[cfg(rev = "647")]
pub const EXPECTED_CRCS: &[Option<i32>] = &[
    Some(0xea49a80a_u32 as i32), // [ 0] anims
    Some(0x30012ff1_u32 as i32), // [ 1] bases
    Some(0xfc33c82a_u32 as i32), // [ 2] config
    Some(0x7cfecb48_u32 as i32), // [ 3] interfaces
    Some(0xe67d0ac1_u32 as i32), // [ 4] synths
    Some(0x364bc307_u32 as i32), // [ 5] maps, with restored donor squares
    Some(0x35c79755_u32 as i32), // [ 6] music
    Some(0x202a3927_u32 as i32), // [ 7] models, with donor additions
    Some(0x746d69f2_u32 as i32), // [ 8] sprites
    Some(0x89dc3959_u32 as i32), // [ 9] textures
    Some(0x2976276a_u32 as i32), // [10] binary
    Some(0xa4581da5_u32 as i32), // [11] jingles
    Some(0xac5b8029_u32 as i32), // [12] clientscripts
    Some(0x986d3e87_u32 as i32), // [13] fontmetrics
    Some(0x9590ca32_u32 as i32), // [14] vorbis
    Some(0x2ec553fe_u32 as i32), // [15] instruments
    Some(0x5749b077_u32 as i32), // [16] config_loc, with donor and authored overlays
    Some(0x01eb855a_u32 as i32), // [17] config_enum
    Some(0x28a3b763_u32 as i32), // [18] config_npc
    Some(0xb1a2e4b3_u32 as i32), // [19] config_obj
    Some(0x77875c75_u32 as i32), // [20] config_seq
    Some(0xa9bd75c4_u32 as i32), // [21] config_spot
    Some(0x7119beac_u32 as i32), // [22] config_varpbit
    Some(0x2bf6b94c_u32 as i32), // [23] worldmap
    Some(0xeee10630_u32 as i32), // [24] quickchat
    Some(0xf9537a6b_u32 as i32), // [25] quickchat_global
    Some(0x674801f9_u32 as i32), // [26] materials
    Some(0x1ef1d58f_u32 as i32), // [27] config_particle
    Some(0x586d0a23_u32 as i32), // [28] defaults
    Some(0x8f918825_u32 as i32), // [29] config_billboard
    Some(0x90d538d0_u32 as i32), // [30] dlls
    Some(0xd397dfdb_u32 as i32), // [31] shaders
    Some(0x907d8a9e_u32 as i32), // [32] loading_sprites
    Some(0x8c760e1c_u32 as i32), // [33] loading_screens
    Some(0xe70e7822_u32 as i32), // [34] loading_sprites_raw
    Some(0x2457425c_u32 as i32), // [35] worldmap_locs (archive id 36; no idx35)
];

#[cfg(rev = "742")]
pub const EXPECTED_CRCS: &[Option<i32>] = &[
    Some(0xc3797935_u32 as i32), // [ 0] anims
    Some(0xcaef1b78_u32 as i32), // [ 1] bases
    Some(0x032772a2_u32 as i32), // [ 2] config
    Some(0x52a519a3_u32 as i32), // [ 3] interfaces
    Some(0x0d8b59df_u32 as i32), // [ 4] synths
    Some(0x2c1df40a_u32 as i32), // [ 5] maps
    Some(0x6d9cf945_u32 as i32), // [ 6] music
    Some(0x01fff9b1_u32 as i32), // [ 7] models
    Some(0x24f541c2_u32 as i32), // [ 8] sprites
    Some(0xebf32062_u32 as i32), // [ 9] textures
    Some(0x1516c462_u32 as i32), // [10] binary
    Some(0x581e86cb_u32 as i32), // [11] jingles
    Some(0x7164700f_u32 as i32), // [12] clientscripts
    Some(0x2f1a3603_u32 as i32), // [13] fontmetrics
    Some(0x4d742ffb_u32 as i32), // [14] vorbis
    Some(0xb58233ca_u32 as i32), // [15] instruments
    Some(0x130823fc_u32 as i32), // [16] config_loc
    Some(0x3c08b310_u32 as i32), // [17] config_enum
    Some(0x0b00d78b_u32 as i32), // [18] config_npc
    Some(0xeefcd4bf_u32 as i32), // [19] config_obj
    Some(0x368548ca_u32 as i32), // [20] config_seq
    Some(0x481c8032_u32 as i32), // [21] config_spot
    Some(0xe25b5923_u32 as i32), // [22] config_varpbit
    Some(0x078c3c61_u32 as i32), // [23] worldmap
    Some(0x3bf8447e_u32 as i32), // [24] quickchat
    Some(0xae3be86d_u32 as i32), // [25] quickchat_global
    Some(0xbb5a85f2_u32 as i32), // [26] materials
    Some(0x7f987d0d_u32 as i32), // [27] config_particle
    Some(0xbabd47f8_u32 as i32), // [28] defaults
    Some(0x0822834b_u32 as i32), // [29] config_billboard
    Some(0x87cc8341_u32 as i32), // [30] dlls
    Some(0x33f1d042_u32 as i32), // [31] shaders
    Some(0xe84ccddd_u32 as i32), // [32] loading_sprites
    Some(0x81357b81_u32 as i32), // [33] loading_screens
    Some(0x1f33f784_u32 as i32), // [34] loading_sprites_raw
    Some(0xc7346710_u32 as i32), // [35] cutscenes
    Some(0xdcfacd02_u32 as i32), // [36] worldmap_locs
    Some(0x83bc7b34_u32 as i32), // [37] textures_hdr
];

/// Quickchat categories and phrases of the global archive carry on from here.
#[cfg(since_530)]
pub const QUICKCHAT_GLOBAL_FIRST: u16 = 0x8000;

/// Mapping from config file names (as produced by the pack step into
/// `data/pack/client/config/`) to their group IDs within archive 2 (CONFIG).
///
/// Config types in archives 16-22 (loc, npc, obj, seq, enum, spotanim,
/// varpbit) are NOT listed here — they have their own archives with strided
/// grouping.
#[cfg(since_530)]
pub const CONFIG_GROUPS: &[(&str, u32)] = &[
    ("flu", CONFIG_GROUP_FLU),
    ("hunt", CONFIG_GROUP_HUNT),
    ("idk", CONFIG_GROUP_IDK),
    ("flo", CONFIG_GROUP_FLO),
    ("inv", CONFIG_GROUP_INV),
    ("mesanim", CONFIG_GROUP_MESANIM),
    ("param", CONFIG_GROUP_PARAM),
    ("varcstr", CONFIG_GROUP_VARCSTR),
    ("varp", CONFIG_GROUP_VARP),
    ("area", CONFIG_GROUP_AREA),
    ("varc", CONFIG_GROUP_VARC),
    ("varobj", CONFIG_GROUP_VAROBJ),
    ("varpstr", CONFIG_GROUP_VARPSTR),
    ("vars", CONFIG_GROUP_VARS),
    ("varsstr", CONFIG_GROUP_VARSSTR),
    ("varn", CONFIG_GROUP_VARN),
    ("struct", CONFIG_GROUP_STRUCT),
    #[cfg(since_647)]
    ("light", CONFIG_GROUP_LIGHT),
    ("bas", CONFIG_GROUP_BAS),
    #[cfg(since_647)]
    ("cursor", CONFIG_GROUP_CURSOR),
    ("msi", CONFIG_GROUP_MSI),
    #[cfg(since_647)]
    ("quest", CONFIG_GROUP_QUEST),
    #[cfg(since_647)]
    ("mel", CONFIG_GROUP_MEL),
    #[cfg(since_647)]
    ("dbtable", CONFIG_GROUP_DBTABLE),
    #[cfg(since_647)]
    ("dbrow", CONFIG_GROUP_DBROW),
    #[cfg(since_647)]
    ("hitmark", CONFIG_GROUP_HITMARK),
    #[cfg(since_742)]
    ("skybox", CONFIG_GROUP_SKYBOX),
    #[cfg(since_742)]
    ("headbar", CONFIG_GROUP_HEADBAR),
    #[cfg(since_647)]
    ("varclansetting", CONFIG_GROUP_VARCLANSETTING),
    #[cfg(since_742)]
    ("varclan", CONFIG_GROUP_VARCLAN),
];

/// Tables the cache holds empty, packed from their files: extension, group,
/// the name a section takes, and the packed table's CRC.
#[cfg(all(since_530, before_742))]
pub const CONFIG_EMPTY_TABLES: &[(&str, u32, &str, i32)] = &[
    #[cfg(rev = "530")]
    ("varnbit", CONFIG_GROUP_VARNBIT, "varnpcbit", 739854011),
    #[cfg(rev = "647")]
    ("varnbit", CONFIG_GROUP_VARNBIT, "varnpcbit", -1298439089),
    #[cfg(since_647)]
    ("config37", 37, "config37", 477744615),
    #[cfg(since_647)]
    ("config38", 38, "config38", -430249901),
    #[cfg(since_647)]
    ("config39", 39, "config39", 583011186),
    #[cfg(since_647)]
    (
        "controller",
        CONFIG_GROUP_CONTROLLER,
        "controller",
        -1179041704,
    ),
    #[cfg(since_647)]
    ("varcon", CONFIG_GROUP_VARCON, "config43", 1930184948),
    #[cfg(since_647)]
    ("varconbit", CONFIG_GROUP_VARCONBIT, "varconbit", 812315341),
    #[cfg(since_647)]
    ("config45", 45, "config45", 260531396),
    #[cfg(since_647)]
    ("varclan", CONFIG_GROUP_VARCLAN, "varclan", -1864459835),
    #[cfg(since_647)]
    ("itemcode", CONFIG_GROUP_ITEMCODE, "itemcode", -430249901),
];
#[cfg(since_742)]
pub const CONFIG_EMPTY_TABLES: &[(&str, u32, &str, i32)] = &[
    ("varnbit", CONFIG_GROUP_VARNBIT, "varnpcbit", -156852319),
    ("config37", 37, "config37", 1873831318),
    ("config38", 38, "config38", 2099745455),
    ("config39", 39, "config39", -1881575280),
    (
        "controller",
        CONFIG_GROUP_CONTROLLER,
        "controller",
        -1806420646,
    ),
    ("varcon", CONFIG_GROUP_VARCON, "config43", 35895006),
    ("varconbit", CONFIG_GROUP_VARCONBIT, "varconbit", -413483330),
    ("config45", 45, "config45", 999201384),
    ("itemcode", CONFIG_GROUP_ITEMCODE, "itemcode", -1228634851),
    ("config49", 49, "config49", -549861796),
    ("config50", 50, "config50", 1360808172),
    ("config51", 51, "config51", 322049397),
    ("config53", 53, "config53", 384300246),
    ("gamelogevent", 70, "gamelogevent", 806050267),
    ("config74", 74, "config74", -1881575280),
];

#[cfg(since_530)]
pub const REF_FORMAT_LEGACY: u8 = 5;
#[cfg(since_530)]
pub const REF_FORMAT_VERSIONED: u8 = 6;
#[cfg(since_742)]
pub const REF_FORMAT_SMART: u8 = 7;

/// The reference table format the revision writes its archives in.
#[cfg(all(since_530, before_742))]
const REF_FORMAT: u8 = REF_FORMAT_VERSIONED;
#[cfg(since_742)]
const REF_FORMAT: u8 = REF_FORMAT_SMART;

/// Static per-archive properties that are fixed for the revision.
///
/// Compression codec is determined per-group by [`compress_best`]: try Gzip
/// and Bzip2, pick whichever payload is smaller. If `allow_uncompressed` is
/// true and compression makes the data larger, fall back to None.
///
/// [`compress_best`]: rs_io::js5::compression::compress_best
#[derive(Debug, Clone, Copy)]
#[cfg(since_530)]
pub struct ArchiveFormat {
    pub id: u8,
    pub format: u8,
    pub named: bool,
    pub has_group_whirlpools: bool,
    pub allow_uncompressed: bool,
    pub allow_bzip2: bool,
    pub content_dir: &'static str,
}

/// All archive IDs present in the cache.
#[cfg(since_530)]
pub const ARCHIVE_IDS: &[u8] = &{
    let mut ids = [0; ARCHIVES.len()];
    let mut i = 0;
    while i < ids.len() {
        ids[i] = ARCHIVES[i].id;
        i += 1;
    }
    ids
};

/// Static format properties for each archive (revision-specific).
#[cfg(since_530)]
pub const ARCHIVES: &[ArchiveFormat] = &[
    //                                                                                     allow_uncompressed  allow_bzip2
    ArchiveFormat {
        id: ANIMS,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "animations",
    },
    ArchiveFormat {
        id: BASES,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "skeletons",
    },
    ArchiveFormat {
        id: CONFIG,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: INTERFACES,
        format: REF_FORMAT,
        named: cfg!(before_742),
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "interfaces",
    },
    ArchiveFormat {
        id: SYNTH,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: true,
        allow_bzip2: false,
        content_dir: "synth",
    },
    ArchiveFormat {
        id: MAPS,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "maps",
    },
    ArchiveFormat {
        id: SONGS,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "songs",
    },
    ArchiveFormat {
        id: MODELS,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "models",
    },
    ArchiveFormat {
        id: SPRITES,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "sprites",
    },
    ArchiveFormat {
        id: TEXTURES,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: true,
        allow_bzip2: cfg!(since_742),
        content_dir: "textures",
    },
    ArchiveFormat {
        id: BINARY,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "binary",
    },
    ArchiveFormat {
        id: JINGLES,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "jingles",
    },
    ArchiveFormat {
        id: CLIENTSCRIPT,
        format: REF_FORMAT,
        named: cfg!(since_647),
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "scripts",
    },
    ArchiveFormat {
        id: FONTMETRICS,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "fontmetrics",
    },
    ArchiveFormat {
        id: VORBIS,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "vorbis",
    },
    ArchiveFormat {
        id: INSTRUMENTS,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: cfg!(since_742),
        content_dir: "instruments",
    },
    ArchiveFormat {
        id: CONFIG_LOC,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_ENUM,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_NPC,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_OBJ,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_SEQ,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_SPOTANIM,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_VARPBIT,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: WORLDMAP,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "worldmap",
    },
    ArchiveFormat {
        id: QUICKCHAT,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "quickchat",
    },
    ArchiveFormat {
        id: QUICKCHAT_GLOBAL,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "quickchat_global",
    },
    ArchiveFormat {
        id: MATERIALS,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "materials",
    },
    ArchiveFormat {
        id: PARTICLES,
        format: if cfg!(since_647) {
            REF_FORMAT
        } else {
            REF_FORMAT_LEGACY
        },
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "particles",
    },
    #[cfg(since_647)]
    ArchiveFormat {
        id: DEFAULTS,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "defaults",
    },
    #[cfg(since_647)]
    ArchiveFormat {
        id: BILLBOARDS,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "billboards",
    },
    #[cfg(since_647)]
    ArchiveFormat {
        id: DLLS,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: true,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "dlls",
    },
    #[cfg(since_647)]
    ArchiveFormat {
        id: SHADERS,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: cfg!(since_742),
        content_dir: "shaders",
    },
    #[cfg(since_647)]
    ArchiveFormat {
        id: LOADING_SPRITES,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "loading_sprites",
    },
    #[cfg(since_647)]
    ArchiveFormat {
        id: LOADING_SCREENS,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "loading_backgrounds",
    },
    #[cfg(since_647)]
    ArchiveFormat {
        id: LOADING_SPRITES_RAW,
        format: REF_FORMAT,
        named: true,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "loading_sprites_raw",
    },
    #[cfg(since_742)]
    ArchiveFormat {
        id: CUTSCENES,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "cutscenes",
    },
    #[cfg(since_647)]
    ArchiveFormat {
        id: WORLDMAP_LOCS,
        format: if cfg!(since_742) {
            REF_FORMAT
        } else {
            REF_FORMAT_LEGACY
        },
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "worldmap_locs",
    },
    #[cfg(since_742)]
    ArchiveFormat {
        id: TEXTURES_HDR,
        format: REF_FORMAT,
        named: false,
        has_group_whirlpools: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "textures_hdr",
    },
];

/// Lookup archive format by ID.
#[cfg(since_530)]
pub fn archive_format(id: u8) -> Option<&'static ArchiveFormat> {
    ARCHIVES.iter().find(|a| a.id == id)
}

/// Jagex codec choices that differ from `compress_best` (try-both-pick-smaller).
/// Derived from full-cache analysis of 93,765 bz2/gz groups across cache 786
/// (build 647). Verified stable across 20 OpenRS2 snapshots (builds 640–650).
/// The since_742 arms are the 742 cache's (OpenRS2 544) exceptions, found by its CRC gate.
#[cfg(since_530)]
pub fn codec_override(archive_id: u8, group_id: u32) -> Option<CompressionType> {
    match (archive_id, group_id) {
        #[cfg(all(since_647, before_742))]
        (CONFIG, CONFIG_GROUP_STRUCT) => Some(CompressionType::Gzip),
        (REFERENCE_TABLES, id) if id == CLIENTSCRIPT as u32 => Some(CompressionType::Gzip),
        #[cfg(all(since_530, before_647))]
        (REFERENCE_TABLES, id) if id == MAPS as u32 || id == MODELS as u32 => {
            Some(CompressionType::Gzip)
        }
        #[cfg(all(since_530, before_647))]
        (INTERFACES, 413 | 539) => Some(CompressionType::None),
        #[cfg(since_742)]
        (REFERENCE_TABLES, id) if id == MAPS as u32 || id == MODELS as u32 => {
            Some(CompressionType::Gzip)
        }
        #[cfg(since_742)]
        (CONFIG_LOC, 196 | 197 | 237 | 267) => Some(CompressionType::Gzip),
        _ => None,
    }
}

/// Runtime version data loaded from `content/versions/`.
#[derive(Debug, Clone)]
#[cfg(since_530)]
pub struct ArchiveVersions {
    pub table_version: u32,
    pub group_versions: HashMap<u32, u32>,
    pub group_file_ids: HashMap<u32, Vec<(u32, Option<i32>)>>,
    pub absent_groups: HashMap<u32, (u32, i32)>,
}

/// All version data keyed by archive ID.
#[cfg(since_530)]
pub type VersionTable = HashMap<u8, ArchiveVersions>;

/// Map archive ID → `.versions` filename stem.
#[cfg(since_530)]
pub fn versions_filename(id: u8) -> Option<&'static str> {
    match id {
        CLIENTSCRIPT => Some("clientscript"),
        CONFIG_LOC => Some("config_loc"),
        CONFIG_ENUM => Some("config_enum"),
        CONFIG_NPC => Some("config_npc"),
        CONFIG_OBJ => Some("config_obj"),
        CONFIG_SEQ => Some("config_seq"),
        CONFIG_SPOTANIM => Some("config_spotanim"),
        CONFIG_VARPBIT => Some("config_varpbit"),
        _ => archive_format(id).map(|format| format.content_dir),
    }
}

/// One `.versions` line: `group_id=version`, plus ` absent_crc=0x...` (and
/// ` name_hash=N` in a named archive) for a group the cache lists without data.
#[cfg(since_530)]
pub fn group_version_line(group: &GroupInfo, absent: bool, named: bool) -> String {
    let mut line = format!("{}={}", group.id, group.version);
    if absent {
        line += &format!(" absent_crc={:#010x}", group.crc);
        if named {
            line += &format!(" name_hash={}", group.name_hash);
        }
    }
    line
}

/// One parsed [`group_version_line`].
#[cfg(since_530)]
pub type GroupVersion = (u32, u32, Option<(u32, i32)>);

/// Reads a [`group_version_line`]: the group, its version, and the CRC and
/// name hash it keeps when absent.
#[cfg(since_530)]
pub fn parse_group_version(line: &str) -> Result<GroupVersion> {
    let (id, rest) = line.split_once('=').context("missing '='")?;
    let mut fields = rest.split_whitespace();
    let version = fields.next().context("missing version")?.parse()?;
    let mut crc = None;
    let mut name_hash = -1;
    for field in fields {
        match field.split_once('=') {
            Some(("absent_crc", value)) => {
                crc = Some(u32::from_str_radix(value.trim_start_matches("0x"), 16)?)
            }
            Some(("name_hash", value)) => name_hash = value.parse()?,
            _ => bail!("unknown field '{field}'"),
        }
    }
    Ok((id.trim().parse()?, version, crc.map(|crc| (crc, name_hash))))
}

/// Load version data from `content/versions/*.versions` text files.
///
/// Each file has a `# table_version=N` header followed by [`group_version_line`]s.
#[cfg(since_530)]
pub fn load_versions_dir(dir: &Path) -> Result<VersionTable> {
    let mut table = VersionTable::new();

    for &archive_id in ARCHIVE_IDS {
        let Some(stem) = versions_filename(archive_id) else {
            continue;
        };
        let path = dir.join(format!("{stem}.versions"));
        if !path.exists() {
            continue;
        }

        let text =
            fs::read_to_string(&path).map_err(|e| anyhow!("reading {}: {e}", path.display()))?;

        let mut table_version = 0u32;
        let mut group_versions = HashMap::new();
        let mut absent_groups = HashMap::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(rest) = line.strip_prefix("# table_version=") {
                table_version = rest
                    .parse()
                    .map_err(|e| anyhow!("{}: bad table_version: {e}", path.display()))?;
                continue;
            }
            if line.starts_with('#') || !line.contains('=') {
                continue;
            }
            let (gid, ver, absent) = parse_group_version(line)
                .with_context(|| format!("{}: bad line '{line}'", path.display()))?;
            group_versions.insert(gid, ver);
            if let Some(absent) = absent {
                absent_groups.insert(gid, absent);
            }
        }

        // Load .fileids (non-trivial file ID + name_hash lists per group)
        // Format: group_id=id[:hash],id[:hash],...
        let mut group_file_ids = HashMap::new();
        let fileids_path = dir.join(format!("{stem}.fileids"));
        if fileids_path.exists() {
            if let Ok(ftext) = fs::read_to_string(&fileids_path) {
                for line in ftext.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    let Some((gid_str, entries_str)) = line.split_once('=') else {
                        continue;
                    };
                    let Ok(gid) = gid_str.trim().parse::<u32>() else {
                        continue;
                    };
                    let files: Vec<(u32, Option<i32>)> = entries_str
                        .split(',')
                        .filter_map(|s| {
                            let s = s.trim();
                            if let Some((id_str, hash_str)) = s.split_once(':') {
                                let id = id_str.parse().ok()?;
                                let hash = hash_str.parse::<i32>().ok();
                                Some((id, hash))
                            } else {
                                Some((s.parse().ok()?, None))
                            }
                        })
                        .collect();
                    if !files.is_empty() {
                        group_file_ids.insert(gid, files);
                    }
                }
            }
        }

        table.insert(
            archive_id,
            ArchiveVersions {
                table_version,
                group_versions,
                group_file_ids,
                absent_groups,
            },
        );
    }

    Ok(table)
}

#[cfg(since_530)]
pub fn verify_crcs(store: &PackedCache, strict: bool) -> Result<()> {
    info!(
        "CRC gate: verifying all {} archives and group payloads...",
        ARCHIVE_IDS.len()
    );
    ensure!(
        store.tables.len() == ARCHIVE_IDS.len(),
        "CRC gate: expected {} reference tables, got {}",
        ARCHIVE_IDS.len(),
        store.tables.len()
    );
    let groups: usize = ARCHIVE_IDS
        .par_iter()
        .enumerate()
        .map(|(pos, &aid)| -> Result<usize> {
            let raw = store.groups.get(&(REFERENCE_TABLES, aid as u32))
                .with_context(|| format!("CRC gate: archive {aid}: missing reference container"))?;
            let expected = EXPECTED_CRCS[pos]
                .with_context(|| format!("CRC gate: archive {aid}: missing expected CRC"))?;
            let actual = crc::getcrc(raw, 0, raw.len());
            if actual != expected {
                ensure!(
                    !strict,
                    "CRC gate: archive {aid}: got {actual:#010x}, expected {expected:#010x}"
                );
                warn!("CRC gate: archive {aid}: got {actual:#010x}, expected {expected:#010x} (verification disabled)");
            }
            let table = store.tables.get(&aid)
                .with_context(|| format!("CRC gate: archive {aid}: missing reference table"))?;
            ensure!(
                encode_index(table) == decompress(raw)?.as_ref(),
                "CRC gate: archive {aid}: reference metadata differs from its container"
            );
            table.groups.par_iter().try_for_each(|group| -> Result<()> {
                let Some(raw) = store.groups.get(&(aid, group.id)) else {
                    ensure!(
                        store.absent.contains(&(aid, group.id)),
                        "CRC gate: archive {aid} group {}: missing payload",
                        group.id
                    );
                    return Ok(());
                };
                let container = Container::parse(raw)
                    .with_context(|| format!("CRC gate: archive {aid} group {}: invalid container", group.id))?;
                ensure!(
                    raw.len() == container.data.len() + 2
                        && container.version == Some(group.version as u16),
                    "CRC gate: archive {aid} group {}: invalid version trailer",
                    group.id
                );
                let actual = container.crc();
                ensure!(
                    actual == group.crc,
                    "CRC gate: archive {aid} group {}: got {actual:#010x}, expected {:#010x}",
                    group.id,
                    group.crc
                );
                if let Some(digest) = group.whirlpool {
                    ensure!(
                        whirlpool(container.data) == digest,
                        "CRC gate: archive {aid} group {}: Whirlpool mismatch",
                        group.id
                    );
                }
                Ok(())
            })?;
            Ok(table.groups.len())
        })
        .sum::<Result<usize>>()?
        - store.absent.len();
    ensure!(
        store.groups.len() == groups + ARCHIVE_IDS.len(),
        "CRC gate: unreferenced containers: expected {}, got {}",
        groups + ARCHIVE_IDS.len(),
        store.groups.len()
    );
    if strict {
        info!(
            "CRC gate: verified {} archives and {groups} group payloads (no skips)",
            ARCHIVE_IDS.len()
        );
    } else {
        info!(
            "Cache integrity: verified {groups} group payloads; reference CRC enforcement disabled"
        );
    }
    Ok(())
}

#[cfg(all(test, since_530))]
mod tests {
    use super::{group_version_line, parse_group_version};
    use rs_io::js5::index::{FileInfo, GroupInfo};

    #[test]
    fn absent_group_lines_keep_their_crc_and_name_hash() {
        let mut group = GroupInfo {
            id: 394,
            name_hash: -1,
            crc: 0x1fe351ce,
            whirlpool: None,
            version: 1,
            size: 0,
            file_count: 1,
            files: vec![FileInfo {
                id: 0,
                name_hash: None,
            }],
        };
        assert_eq!(group_version_line(&group, false, false), "394=1");
        assert_eq!(parse_group_version("394=1").unwrap(), (394, 1, None));
        let line = group_version_line(&group, true, false);
        assert_eq!(line, "394=1 absent_crc=0x1fe351ce");
        assert_eq!(
            parse_group_version(&line).unwrap(),
            (394, 1, Some((0x1fe351ce, -1)))
        );
        group.name_hash = -12345;
        let line = group_version_line(&group, true, true);
        assert_eq!(line, "394=1 absent_crc=0x1fe351ce name_hash=-12345");
        assert_eq!(
            parse_group_version(&line).unwrap(),
            (394, 1, Some((0x1fe351ce, -12345)))
        );
        assert!(parse_group_version("394=1 crc=0x1").is_err());
    }
}
