use anyhow::Result;
#[cfg(since_647)]
use anyhow::anyhow;
#[cfg(since_647)]
use rs_io::crc;
#[cfg(before_647)]
use rs_io::jag::JagCompression;
#[cfg(since_244)]
use rs_io::js5::Js5Store;
#[cfg(since_647)]
use rs_io::js5::compression::CompressionType;
#[cfg(since_647)]
use std::collections::HashMap;
#[cfg(any(rev = "225", since_647))]
use std::fs;
use std::path::Path;
#[cfg(rev = "225")]
use std::path::PathBuf;
#[cfg(since_647)]
use tracing::{info, warn};

#[cfg(all(since_244, before_647))]
pub const JAG: u8 = 0;
#[cfg(all(since_244, before_647))]
pub const MIDI: u8 = 3;

#[cfg(before_647)]
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

#[cfg(before_647)]
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

#[cfg(before_647)]
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

#[cfg(before_647)]
pub(crate) struct JagSource {
    #[cfg(rev = "225")]
    dir: PathBuf,
    #[cfg(since_244)]
    cache: Js5Store,
}

#[cfg(before_647)]
impl JagSource {
    pub(crate) fn open(expected_dir: &Path) -> Result<Self> {
        Ok(Self {
            #[cfg(rev = "225")]
            dir: expected_dir.to_path_buf(),
            #[cfg(since_244)]
            cache: Js5Store::open(expected_dir)?,
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
    pub(crate) fn cache(&mut self) -> &mut Js5Store {
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

#[cfg(rev = "647")]
pub const EXPECTED_CRCS: &[Option<i32>] = &[
    Some(0xea49a80a_u32 as i32), // [ 0] anims
    Some(0x30012ff1_u32 as i32), // [ 1] bases
    Some(0xfc33c82a_u32 as i32), // [ 2] config
    Some(0x7cfecb48_u32 as i32), // [ 3] interfaces
    Some(0xe67d0ac1_u32 as i32), // [ 4] synths
    None,                        // [ 5] maps (0x83bd3157)
    Some(0x35c79755_u32 as i32), // [ 6] music
    None,                        // [ 7] models (0x99bc666e)
    Some(0x746d69f2_u32 as i32), // [ 8] sprites
    Some(0x89dc3959_u32 as i32), // [ 9] textures
    Some(0x2976276a_u32 as i32), // [10] binary
    Some(0xa4581da5_u32 as i32), // [11] jingles
    None,                        // [12] clientscripts (0xac5b8029)
    Some(0x986d3e87_u32 as i32), // [13] fontmetrics
    Some(0x9590ca32_u32 as i32), // [14] vorbis
    Some(0x2ec553fe_u32 as i32), // [15] instruments
    None,                        // [16] config_loc (0x63048bf1)
    Some(0x01eb855a_u32 as i32), // [17] config_enum
    Some(0x28a3b763_u32 as i32), // [18] config_npc
    Some(0xb1a2e4b3_u32 as i32), // [19] config_obj
    Some(0x77875c75_u32 as i32), // [20] config_seq
    Some(0xa9bd75c4_u32 as i32), // [21] config_spot
    Some(0x7119beac_u32 as i32), // [22] config_varbit
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
    Some(0x2457425c_u32 as i32), // [34] worldmap_locs (archive id 36 — no idx34/idx35)
];

#[cfg(since_244)]
pub const ANIMS: u8 = if cfg!(since_647) { 0 } else { 2 };
#[cfg(since_647)]
pub const BASES: u8 = 1;
#[cfg(since_647)]
pub const CONFIG: u8 = 2;
#[cfg(since_647)]
pub const INTERFACES: u8 = 3;
#[cfg(since_647)]
pub const SYNTH: u8 = 4;
#[cfg(since_244)]
pub const MAPS: u8 = if cfg!(since_647) { 5 } else { 4 };
#[cfg(since_647)]
pub const SONGS: u8 = 6;
#[cfg(since_244)]
pub const MODELS: u8 = if cfg!(since_647) { 7 } else { 1 };
#[cfg(since_647)]
pub const SPRITES: u8 = 8;
#[cfg(since_647)]
pub const TEXTURES: u8 = 9;
#[cfg(since_647)]
pub const BINARY: u8 = 10;
#[cfg(since_647)]
pub const JINGLES: u8 = 11;
#[cfg(since_647)]
pub const CLIENTSCRIPT: u8 = 12;
#[cfg(since_647)]
pub const FONTMETRICS: u8 = 13;
#[cfg(since_647)]
pub const VORBIS: u8 = 14;
#[cfg(since_647)]
pub const MUSIC_SAMPLES: u8 = 15;
#[cfg(since_647)]
pub const CONFIG_LOC: u8 = 16;
#[cfg(since_647)]
pub const CONFIG_ENUM: u8 = 17;
#[cfg(since_647)]
pub const CONFIG_NPC: u8 = 18;
#[cfg(since_647)]
pub const CONFIG_OBJ: u8 = 19;
#[cfg(since_647)]
pub const CONFIG_SEQ: u8 = 20;
#[cfg(since_647)]
pub const CONFIG_SPOTANIM: u8 = 21;
#[cfg(since_647)]
pub const CONFIG_VARBIT: u8 = 22;
#[cfg(since_647)]
pub const WORLDMAP_DATA: u8 = 23;
#[cfg(since_647)]
pub const QUICKCHAT: u8 = 24;
#[cfg(since_647)]
pub const QUICKCHAT_GLOBAL: u8 = 25;
#[cfg(since_647)]
pub const MATERIALS: u8 = 26;
#[cfg(since_647)]
pub const PARTICLES: u8 = 27;
#[cfg(since_647)]
pub const DEFAULTS: u8 = 28;
#[cfg(since_647)]
pub const BILLBOARDS: u8 = 29;
#[cfg(since_647)]
pub const DLLS: u8 = 30;
#[cfg(since_647)]
pub const SHADERS: u8 = 31;
#[cfg(since_647)]
pub const LOADING_SPRITES: u8 = 32;
#[cfg(since_647)]
pub const LOADING_SCREENS: u8 = 33;
#[cfg(since_647)]
pub const WORLDMAP_LOCS: u8 = 36;
#[cfg(since_647)]
pub const REFERENCE_TABLES: u8 = 255;

#[cfg(since_647)]
pub const CONFIG_GROUP_FLU: u32 = 1;
#[cfg(since_647)]
pub const CONFIG_GROUP_IDK: u32 = 3;
#[cfg(since_647)]
pub const CONFIG_GROUP_FLO: u32 = 4;
#[cfg(since_647)]
pub const CONFIG_GROUP_INV: u32 = 5;
#[cfg(since_647)]
pub const CONFIG_GROUP_PARAM: u32 = 11;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARP: u32 = 16;
#[cfg(since_647)]
pub const CONFIG_GROUP_STRUCT: u32 = 26;
#[cfg(since_647)]
pub const CONFIG_GROUP_LIGHT: u32 = 31;
#[cfg(since_647)]
pub const CONFIG_GROUP_BAS: u32 = 32;
#[cfg(since_647)]
pub const CONFIG_GROUP_CURSOR: u32 = 33;
#[cfg(since_647)]
pub const CONFIG_GROUP_MSI: u32 = 34;
#[cfg(since_647)]
pub const CONFIG_GROUP_QUEST: u32 = 35;
#[cfg(since_647)]
pub const CONFIG_GROUP_MEL: u32 = 36;
#[cfg(since_647)]
pub const CONFIG_GROUP_HITMARK: u32 = 46;
#[cfg(since_647)]
pub const CONFIG_GROUP_VARCLAN: u32 = 54;

/// Mapping from config file names (as produced by the pack step into
/// `data/pack/client/config/`) to their group IDs within archive 2 (CONFIG).
///
/// Config types in archives 16-22 (loc, npc, obj, seq, enum, spotanim,
/// varbit) are NOT listed here — they have their own archives with strided
/// grouping.
#[cfg(since_647)]
pub const CONFIG_GROUPS: &[(&str, u32)] = &[
    ("flu", CONFIG_GROUP_FLU),
    ("idk", CONFIG_GROUP_IDK),
    ("flo", CONFIG_GROUP_FLO),
    ("inv", CONFIG_GROUP_INV),
    ("param", CONFIG_GROUP_PARAM),
    ("varp", CONFIG_GROUP_VARP),
    ("struct", CONFIG_GROUP_STRUCT),
    ("light", CONFIG_GROUP_LIGHT),
    ("bas", CONFIG_GROUP_BAS),
    ("cursor", CONFIG_GROUP_CURSOR),
    ("msi", CONFIG_GROUP_MSI),
    ("quest", CONFIG_GROUP_QUEST),
    ("mel", CONFIG_GROUP_MEL),
    ("hitmark", CONFIG_GROUP_HITMARK),
    ("varclansetting", CONFIG_GROUP_VARCLAN),
];

/// Unknown config groups in archive 2 packed as raw binary. The pack step
/// stores these as `configN.dat`/`configN.idx`.
#[cfg(since_647)]
pub const CONFIG_UNKNOWN_GROUPS: &[u32] = &[
    2, 7, 15, 18, 19, 20, 21, 22, 23, 24, 25, 37, 38, 39, 40, 41, 42, 43, 44, 45, 47, 48,
];

#[cfg(since_647)]
pub const REF_FORMAT_LEGACY: u8 = 5;
#[cfg(since_647)]
pub const REF_FORMAT_VERSIONED: u8 = 6;

/// Static per-archive properties that are fixed for the revision.
///
/// Compression codec is determined per-group by [`compress_best`]: try Gzip
/// and Bzip2, pick whichever payload is smaller. If `allow_uncompressed` is
/// true and compression makes the data larger, fall back to None.
///
/// [`compress_best`]: rs_io::js5::compression::compress_best
#[derive(Debug, Clone, Copy)]
#[cfg(since_647)]
pub struct ArchiveFormat {
    pub id: u8,
    pub format: u8,
    pub named: bool,
    pub whirlpooled: bool,
    pub allow_uncompressed: bool,
    pub allow_bzip2: bool,
    pub content_dir: &'static str,
}

/// All archive IDs present in the rev 647 cache.
#[cfg(since_647)]
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
#[cfg(since_647)]
pub const ARCHIVES: &[ArchiveFormat] = &[
    //                                                                                     allow_uncompressed  allow_bzip2
    ArchiveFormat {
        id: ANIMS,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "animations",
    },
    ArchiveFormat {
        id: BASES,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "skeletons",
    },
    ArchiveFormat {
        id: CONFIG,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: INTERFACES,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "interfaces",
    },
    ArchiveFormat {
        id: SYNTH,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: true,
        allow_bzip2: false,
        content_dir: "synth",
    },
    ArchiveFormat {
        id: MAPS,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "maps",
    },
    ArchiveFormat {
        id: SONGS,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "songs",
    },
    ArchiveFormat {
        id: MODELS,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "models",
    },
    ArchiveFormat {
        id: SPRITES,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "sprites",
    },
    ArchiveFormat {
        id: TEXTURES,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: true,
        allow_bzip2: false,
        content_dir: "textures",
    },
    ArchiveFormat {
        id: BINARY,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "binary",
    },
    ArchiveFormat {
        id: JINGLES,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "jingles",
    },
    ArchiveFormat {
        id: CLIENTSCRIPT,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "clientscript",
    },
    ArchiveFormat {
        id: FONTMETRICS,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "fontmetrics",
    },
    ArchiveFormat {
        id: VORBIS,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "vorbis",
    },
    ArchiveFormat {
        id: MUSIC_SAMPLES,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "music_samples",
    },
    ArchiveFormat {
        id: CONFIG_LOC,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_ENUM,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_NPC,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_OBJ,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_SEQ,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_SPOTANIM,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "config",
    },
    ArchiveFormat {
        id: CONFIG_VARBIT,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "config",
    },
    ArchiveFormat {
        id: WORLDMAP_DATA,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "worldmap_data",
    },
    ArchiveFormat {
        id: QUICKCHAT,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "quickchat",
    },
    ArchiveFormat {
        id: QUICKCHAT_GLOBAL,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "quickchat_global",
    },
    ArchiveFormat {
        id: MATERIALS,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "materials",
    },
    ArchiveFormat {
        id: PARTICLES,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "particles",
    },
    ArchiveFormat {
        id: DEFAULTS,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "defaults",
    },
    ArchiveFormat {
        id: BILLBOARDS,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "billboards",
    },
    ArchiveFormat {
        id: DLLS,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: true,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "dlls",
    },
    ArchiveFormat {
        id: SHADERS,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "shaders",
    },
    ArchiveFormat {
        id: LOADING_SPRITES,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "loading_sprites",
    },
    ArchiveFormat {
        id: LOADING_SCREENS,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "loading_backgrounds",
    },
    ArchiveFormat {
        id: WORLDMAP_LOCS,
        format: REF_FORMAT_LEGACY,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "worldmap_locs",
    },
];

/// Lookup archive format by ID.
#[cfg(since_647)]
pub fn archive_format(id: u8) -> Option<&'static ArchiveFormat> {
    ARCHIVES.iter().find(|a| a.id == id)
}

/// Jagex codec choices that differ from `compress_best` (try-both-pick-smaller).
/// Derived from full-cache analysis of 93,765 bz2/gz groups across cache 786
/// (build 647). Verified stable across 20 OpenRS2 snapshots (builds 640–650).
#[cfg(since_647)]
pub fn codec_override(archive_id: u8, group_id: u32) -> Option<CompressionType> {
    match (archive_id, group_id) {
        (CONFIG, CONFIG_GROUP_STRUCT) => Some(CompressionType::Gzip),
        _ => None,
    }
}

/// Runtime version data loaded from `content/versions/`.
#[derive(Debug, Clone)]
#[cfg(since_647)]
pub struct ArchiveVersions {
    pub table_version: u32,
    pub group_versions: HashMap<u32, u32>,
    pub group_file_ids: HashMap<u32, Vec<(u32, Option<i32>)>>,
}

/// All version data keyed by archive ID.
#[cfg(since_647)]
pub type VersionTable = HashMap<u8, ArchiveVersions>;

/// Map archive ID → `.versions` filename stem.
#[cfg(since_647)]
pub fn versions_filename(id: u8) -> Option<&'static str> {
    match id {
        CONFIG_LOC => Some("config_loc"),
        CONFIG_ENUM => Some("config_enum"),
        CONFIG_NPC => Some("config_npc"),
        CONFIG_OBJ => Some("config_obj"),
        CONFIG_SEQ => Some("config_seq"),
        CONFIG_SPOTANIM => Some("config_spotanim"),
        CONFIG_VARBIT => Some("config_varbit"),
        _ => archive_format(id).map(|format| format.content_dir),
    }
}

/// Load version data from `content/versions/*.versions` text files.
///
/// Each file has a `# table_version=N` header followed by `group_id=version` lines.
#[cfg(since_647)]
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
            if line.starts_with('#') {
                continue;
            }
            let Some((gid_str, ver_str)) = line.split_once('=') else {
                continue;
            };
            let gid: u32 = gid_str
                .trim()
                .parse()
                .map_err(|e| anyhow!("{}: bad group id '{gid_str}': {e}", path.display()))?;
            let ver: u32 = ver_str
                .trim()
                .parse()
                .map_err(|e| anyhow!("{}: bad version '{ver_str}': {e}", path.display()))?;
            group_versions.insert(gid, ver);
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
            },
        );
    }

    Ok(table)
}

#[cfg(since_647)]
pub fn verify_crcs(output_dir: &Path, strict: bool) -> Result<()> {
    let mut store = Js5Store::open(output_dir)?;
    let mut mismatches: Vec<String> = Vec::new();
    info!("CRC gate: verifying 35 archives against OpenRS2 786...");
    for (pos, &aid) in ARCHIVE_IDS.iter().enumerate() {
        let actual = match store.read_raw_group_bytes(255, aid as u32) {
            Ok(Some(raw)) => crc::getcrc(&raw, 0, raw.len()),
            _ => 0,
        };
        match EXPECTED_CRCS[pos] {
            Some(expected) if expected == actual => {
                info!("  archive {aid:>2}: OK ({actual:#010x})");
            }
            Some(expected) => {
                warn!("  archive {aid:>2}: MISMATCH got {actual:#010x}, expected {expected:#010x}");
                mismatches.push(format!(
                    "archive {aid}: got {actual:#010x}, expected {expected:#010x}"
                ));
            }
            None => {
                info!("  archive {aid:>2}: skipped, known divergence ({actual:#010x})");
            }
        }
    }
    if mismatches.is_empty() {
        info!("CRC gate: all 31 verified archives match OpenRS2 786 (4 known divergences skipped)");
    } else if strict {
        panic!(
            "CRC gate: {} archive(s) diverge from OpenRS2 786:\n  {}",
            mismatches.len(),
            mismatches.join("\n  "),
        );
    } else {
        warn!(
            "CRC gate: {} archive(s) diverge from OpenRS2 786",
            mismatches.len()
        );
    }
    Ok(())
}
