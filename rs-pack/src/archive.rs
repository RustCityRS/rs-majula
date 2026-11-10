#[cfg(since_647)]
use crate::cache::js5::PackedCache;
use anyhow::Result;
#[cfg(since_647)]
use anyhow::{Context, anyhow, ensure};
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(since_647)]
use rs_crypto::whirlpool;
#[cfg(since_647)]
use rs_io::crc;
#[cfg(before_647)]
use rs_io::jag::JagCompression;
#[cfg(all(since_244, before_647))]
use rs_io::js5::Js5Store;
#[cfg(since_647)]
use rs_io::js5::compression::{CompressionType, Container, decompress};
#[cfg(since_647)]
use rs_io::js5::reference::encode_reference_table;
#[cfg(since_647)]
use std::collections::HashMap;
#[cfg(any(rev = "225", since_647))]
use std::fs;
use std::path::Path;
#[cfg(rev = "225")]
use std::path::PathBuf;
#[cfg(since_647)]
use tracing::{info, warn};

#[cfg(since_244)]
pub use rs_ver::archive::*;

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
pub(crate) fn config_jag_name(kind: &str) -> &str {
    if kind == "varpbit" { "varbit" } else { kind }
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
    Some(0xe70e7822_u32 as i32), // [34] loading_sprites_raw
    Some(0x2457425c_u32 as i32), // [35] worldmap_locs (archive id 36; no idx35)
];

/// Quickchat categories and phrases of the global archive carry on from here.
#[cfg(since_647)]
pub const QUICKCHAT_GLOBAL_FIRST: u16 = 0x8000;

/// Mapping from config file names (as produced by the pack step into
/// `data/pack/client/config/`) to their group IDs within archive 2 (CONFIG).
///
/// Config types in archives 16-22 (loc, npc, obj, seq, enum, spotanim,
/// varbit) are NOT listed here — they have their own archives with strided
/// grouping.
#[cfg(since_647)]
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
    ("light", CONFIG_GROUP_LIGHT),
    ("bas", CONFIG_GROUP_BAS),
    ("cursor", CONFIG_GROUP_CURSOR),
    ("msi", CONFIG_GROUP_MSI),
    ("quest", CONFIG_GROUP_QUEST),
    ("mel", CONFIG_GROUP_MEL),
    ("dbtable", CONFIG_GROUP_DBTABLE),
    ("dbrow", CONFIG_GROUP_DBROW),
    ("hitmark", CONFIG_GROUP_HITMARK),
    ("varclansetting", CONFIG_GROUP_VARCLANSETTING),
];

/// Tables the cache holds empty, packed from their files: extension, group,
/// the name a section takes, and the packed table's CRC.
#[cfg(since_647)]
pub const CONFIG_EMPTY_TABLES: &[(&str, u32, &str, i32)] = &[
    ("varnbit", CONFIG_GROUP_VARNBIT, "varnpcbit", -1298439089),
    ("config37", 37, "config37", 477744615),
    ("config38", 38, "config38", -430249901),
    ("config39", 39, "config39", 583011186),
    (
        "controller",
        CONFIG_GROUP_CONTROLLER,
        "controller",
        -1179041704,
    ),
    ("varcon", CONFIG_GROUP_VARCON, "config43", 1930184948),
    ("varconbit", CONFIG_GROUP_VARCONBIT, "varconbit", 812315341),
    ("config45", 45, "config45", 260531396),
    ("varclan", CONFIG_GROUP_VARCLAN, "varclan", -1864459835),
    ("itemcode", CONFIG_GROUP_ITEMCODE, "itemcode", -430249901),
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
        content_dir: "scripts",
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
        id: INSTRUMENTS,
        format: REF_FORMAT_VERSIONED,
        named: false,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: false,
        content_dir: "instruments",
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
        id: WORLDMAP,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: true,
        allow_bzip2: true,
        content_dir: "worldmap",
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
        id: LOADING_SPRITES_RAW,
        format: REF_FORMAT_VERSIONED,
        named: true,
        whirlpooled: false,
        allow_uncompressed: false,
        allow_bzip2: true,
        content_dir: "loading_sprites_raw",
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
        (REFERENCE_TABLES, id) if id == CLIENTSCRIPT as u32 => Some(CompressionType::Gzip),
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
        CLIENTSCRIPT => Some("clientscript"),
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
                encode_reference_table(table) == decompress(raw)?.as_ref(),
                "CRC gate: archive {aid}: reference metadata differs from its container"
            );
            table.groups.par_iter().try_for_each(|group| -> Result<()> {
                let raw = store.groups.get(&(aid, group.id))
                    .with_context(|| format!("CRC gate: archive {aid} group {}: missing payload", group.id))?;
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
        .sum::<Result<usize>>()?;
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
