//! Rev 647 JS5 archive definitions.
//!
//! Every constant needed to build a JS5 cache from source lives here:
//! archive IDs, reference table format, compression type, content paths.
//! Table versions live in `content/versions.json`.

use std::collections::HashMap;
use std::path::Path;

// ── Archive IDs ────────────────────────────────────────────────────────

pub const ANIMS: u8 = 0;
pub const BASES: u8 = 1;
pub const CONFIG: u8 = 2;
pub const INTERFACES: u8 = 3;
pub const SYNTH: u8 = 4;
pub const MAPS: u8 = 5;
pub const SONGS: u8 = 6;
pub const MODELS: u8 = 7;
pub const SPRITES: u8 = 8;
pub const TEXTURES: u8 = 9;
pub const BINARY: u8 = 10;
pub const JINGLES: u8 = 11;
pub const CLIENTSCRIPT: u8 = 12;
pub const FONTMETRICS: u8 = 13;
pub const VORBIS: u8 = 14;
pub const MUSIC_SAMPLES: u8 = 15;
pub const CONFIG_LOC: u8 = 16;
pub const CONFIG_ENUM: u8 = 17;
pub const CONFIG_NPC: u8 = 18;
pub const CONFIG_OBJ: u8 = 19;
pub const CONFIG_SEQ: u8 = 20;
pub const CONFIG_SPOTANIM: u8 = 21;
pub const CONFIG_VARBIT: u8 = 22;
pub const WORLDMAP_DATA: u8 = 23;
pub const QUICKCHAT: u8 = 24;
pub const QUICKCHAT_GLOBAL: u8 = 25;
pub const MATERIALS: u8 = 26;
pub const PARTICLES: u8 = 27;
pub const DEFAULTS: u8 = 28;
pub const BILLBOARDS: u8 = 29;
pub const DLLS: u8 = 30;
pub const SHADERS: u8 = 31;
pub const LOADING_SPRITES: u8 = 32;
pub const LOADING_SCREENS: u8 = 33;
pub const WORLDMAP_LOCS: u8 = 36;
pub const REFERENCE_TABLES: u8 = 255;

// ── Config group IDs within archive 2 ──────────────────────────────────

pub const CONFIG_GROUP_FLU: u32 = 1;
pub const CONFIG_GROUP_HUNT: u32 = 2;
pub const CONFIG_GROUP_IDK: u32 = 3;
pub const CONFIG_GROUP_FLO: u32 = 4;
pub const CONFIG_GROUP_INV: u32 = 5;
pub const CONFIG_GROUP_PARAM: u32 = 11;
pub const CONFIG_GROUP_VARP: u32 = 16;
pub const CONFIG_GROUP_STRUCT: u32 = 26;
pub const CONFIG_GROUP_LIGHT: u32 = 31;
pub const CONFIG_GROUP_BAS: u32 = 32;
pub const CONFIG_GROUP_CURSOR: u32 = 33;
pub const CONFIG_GROUP_MSI: u32 = 34;
pub const CONFIG_GROUP_QUEST: u32 = 35;
pub const CONFIG_GROUP_MEL: u32 = 36;
pub const CONFIG_GROUP_HITMARK: u32 = 46;
pub const CONFIG_GROUP_VARCLAN: u32 = 54;

/// Mapping from config file names (as produced by the pack step into
/// `data/pack/client/config/`) to their group IDs within archive 2 (CONFIG).
///
/// Config types in archives 16-22 (loc, npc, obj, seq, enum, spotanim,
/// varbit) are NOT listed here — they have their own archives with strided
/// grouping.
pub const CONFIG_GROUPS: &[(&str, u32)] = &[
    ("flu", CONFIG_GROUP_FLU),
    ("hunt", CONFIG_GROUP_HUNT),
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
pub const CONFIG_UNKNOWN_GROUPS: &[u32] = &[
    7, 15, 18, 19, 20, 21, 22, 23, 24, 25, 37, 38, 39, 40, 41, 42, 43, 44, 45, 47, 48,
];

// ── Reference table format versions ────────────────────────────────────

pub const REF_FORMAT_LEGACY: u8 = 5;
pub const REF_FORMAT_VERSIONED: u8 = 6;

// ── Archive metadata ───────────────────────────────────────────────────

/// Static per-archive properties that are fixed for the revision.
///
/// Compression codec is determined per-group by [`compress_best`]: try Gzip
/// and Bzip2, pick whichever payload is smaller. If `allow_uncompressed` is
/// true and compression makes the data larger, fall back to None.
///
/// [`compress_best`]: crate::compression::compress_best
#[derive(Debug, Clone, Copy)]
pub struct ArchiveFormat {
    pub id: u8,
    /// Reference table format version.
    pub format: u8,
    /// Whether groups have name hashes in the reference table.
    pub named: bool,
    /// Whether groups have whirlpool digests in the reference table.
    pub whirlpooled: bool,
    /// Whether groups may be stored uncompressed (codec=None).
    /// When false, groups are always compressed even if it increases size.
    pub allow_uncompressed: bool,
    /// Whether bzip2 is a candidate codec for this archive.
    /// When false, groups always use gzip (Jagex's build tool never
    /// produces bzip2 for certain archives). Empirically verified across
    /// 99,664 groups: archives 4-7, 9-12, 15, 21, 31, 33 use gzip
    /// exclusively.
    pub allow_bzip2: bool,
    /// Content directory name under `content/` (e.g. "animations").
    pub content_dir: &'static str,
}

/// All archive IDs present in the rev 647 cache.
pub const ARCHIVE_IDS: &[u8] = &[
    ANIMS,
    BASES,
    CONFIG,
    INTERFACES,
    SYNTH,
    MAPS,
    SONGS,
    MODELS,
    SPRITES,
    TEXTURES,
    BINARY,
    JINGLES,
    CLIENTSCRIPT,
    FONTMETRICS,
    VORBIS,
    MUSIC_SAMPLES,
    CONFIG_LOC,
    CONFIG_ENUM,
    CONFIG_NPC,
    CONFIG_OBJ,
    CONFIG_SEQ,
    CONFIG_SPOTANIM,
    CONFIG_VARBIT,
    WORLDMAP_DATA,
    QUICKCHAT,
    QUICKCHAT_GLOBAL,
    MATERIALS,
    PARTICLES,
    DEFAULTS,
    BILLBOARDS,
    DLLS,
    SHADERS,
    LOADING_SPRITES,
    LOADING_SCREENS,
    WORLDMAP_LOCS,
];

/// Static format properties for each archive (revision-specific).
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
pub fn archive_format(id: u8) -> Option<&'static ArchiveFormat> {
    ARCHIVES.iter().find(|a| a.id == id)
}

// ── Codec overrides ───────────────────────────────────────────────────

/// Jagex codec choices that differ from `compress_best` (try-both-pick-smaller).
/// Derived from full-cache analysis of 93,765 bz2/gz groups across cache 786
/// (build 647). Verified stable across 20 OpenRS2 snapshots (builds 640–650).
pub fn codec_override(
    archive_id: u8,
    group_id: u32,
) -> Option<crate::compression::CompressionType> {
    match (archive_id, group_id) {
        (CONFIG, 26) => Some(crate::compression::CompressionType::Gzip),
        _ => None,
    }
}

// ── Backwards compatibility ────────────────────────────────────────────

/// Alias for [`ARCHIVES`] — kept for existing callers during migration.
pub const ARCHIVE_FORMATS: &[ArchiveFormat] = ARCHIVES;

// ── Version data ───────────────────────────────────────────────────────

/// Per-group metadata from `content/versions.json`.
#[derive(Debug, Clone)]
pub struct GroupMeta {
    pub version: u32,
    pub file_count: u32,
}

/// Runtime version data loaded from `content/versions/`.
#[derive(Debug, Clone)]
pub struct ArchiveVersions {
    pub table_version: u32,
    pub group_versions: HashMap<u32, u32>,
    pub group_meta: HashMap<u32, GroupMeta>,
    pub group_file_ids: HashMap<u32, Vec<(u32, Option<i32>)>>,
}

/// All version data keyed by archive ID.
pub type VersionTable = HashMap<u8, ArchiveVersions>;

/// Map archive ID → `.versions` filename stem.
pub fn versions_filename(id: u8) -> Option<&'static str> {
    match id {
        ANIMS => Some("animations"),
        BASES => Some("skeletons"),
        CONFIG => Some("config"),
        INTERFACES => Some("interfaces"),
        SYNTH => Some("synth"),
        MAPS => Some("maps"),
        SONGS => Some("songs"),
        MODELS => Some("models"),
        SPRITES => Some("sprites"),
        TEXTURES => Some("textures"),
        BINARY => Some("binary"),
        JINGLES => Some("jingles"),
        CLIENTSCRIPT => Some("clientscript"),
        FONTMETRICS => Some("fontmetrics"),
        VORBIS => Some("vorbis"),
        MUSIC_SAMPLES => Some("music_samples"),
        CONFIG_LOC => Some("config_loc"),
        CONFIG_ENUM => Some("config_enum"),
        CONFIG_NPC => Some("config_npc"),
        CONFIG_OBJ => Some("config_obj"),
        CONFIG_SEQ => Some("config_seq"),
        CONFIG_SPOTANIM => Some("config_spotanim"),
        CONFIG_VARBIT => Some("config_varbit"),
        WORLDMAP_DATA => Some("worldmap_data"),
        QUICKCHAT => Some("quickchat"),
        QUICKCHAT_GLOBAL => Some("quickchat_global"),
        MATERIALS => Some("materials"),
        PARTICLES => Some("particles"),
        DEFAULTS => Some("defaults"),
        BILLBOARDS => Some("billboards"),
        DLLS => Some("dlls"),
        SHADERS => Some("shaders"),
        LOADING_SPRITES => Some("loading_sprites"),
        LOADING_SCREENS => Some("loading_backgrounds"),
        WORLDMAP_LOCS => Some("worldmap_locs"),
        _ => None,
    }
}

/// Load version data from `content/versions/*.versions` text files.
///
/// Each file has a `# table_version=N` header followed by `group_id=version` lines.
pub fn load_versions_dir(dir: &Path) -> anyhow::Result<VersionTable> {
    let mut table = VersionTable::new();

    for &archive_id in ARCHIVE_IDS {
        let Some(stem) = versions_filename(archive_id) else {
            continue;
        };
        let path = dir.join(format!("{stem}.versions"));
        if !path.exists() {
            continue;
        }

        let text = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;

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
                    .map_err(|e| anyhow::anyhow!("{}: bad table_version: {e}", path.display()))?;
                continue;
            }
            if line.starts_with('#') {
                continue;
            }
            let Some((gid_str, ver_str)) = line.split_once('=') else {
                continue;
            };
            let gid: u32 = gid_str.trim().parse().map_err(|e| {
                anyhow::anyhow!("{}: bad group id '{gid_str}': {e}", path.display())
            })?;
            let ver: u32 = ver_str
                .trim()
                .parse()
                .map_err(|e| anyhow::anyhow!("{}: bad version '{ver_str}': {e}", path.display()))?;
            group_versions.insert(gid, ver);
        }

        let group_meta = group_versions
            .iter()
            .map(|(&gid, &ver)| {
                (
                    gid,
                    GroupMeta {
                        version: ver,
                        file_count: 1,
                    },
                )
            })
            .collect();

        // Load .fileids (non-trivial file ID + name_hash lists per group)
        // Format: group_id=id[:hash],id[:hash],...
        let mut group_file_ids = HashMap::new();
        let fileids_path = dir.join(format!("{stem}.fileids"));
        if fileids_path.exists() {
            if let Ok(ftext) = std::fs::read_to_string(&fileids_path) {
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
                group_meta,
                group_file_ids,
            },
        );
    }

    Ok(table)
}
