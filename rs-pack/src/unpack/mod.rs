pub mod config;
pub mod configdiff;
pub mod interface;
pub mod map;
pub mod media;
pub mod model;
mod namecrack;
pub mod report;
pub mod song;
pub mod sound;
pub mod texture;
pub mod title;
pub mod wordenc;

use crate::pack::pack::pack_assets;
use crate::pack::pack_registry::{PackRegistry, PackedFile};
#[cfg(since_244)]
use crate::versionlist::VersionListMeta;
#[cfg(since_244)]
use crate::versionlist::{TABLE_NAMES, VersionList, read_i32_table, read_u16_table};
use crate::{CONTENT_DIR, PACK_DIR, config_crc, jag_crc};
#[cfg(since_244)]
use model::build_model_textures;
use report::{CrcReport, PackDiffReport};
use rs_io::Packet;
use rs_io::jag::{JagCompression, JagFile};
#[cfg(since_244)]
use rs_io::js5::Js5Store;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
#[cfg(rev = "225")]
use std::path::PathBuf;
use std::rc::Rc;
use tracing::{debug, warn};

const CONFIG_TYPE_CRCS: &[(&str, i32)] = &[
    ("seq", config_crc::SEQ),
    ("loc", config_crc::LOC),
    ("flo", config_crc::FLO),
    ("spotanim", config_crc::SPOTANIM),
    ("obj", config_crc::OBJ),
    ("npc", config_crc::NPC),
    ("idk", config_crc::IDK),
    ("varp", config_crc::VARP),
    #[cfg(since_254)]
    ("varbit", config_crc::VARBIT),
    #[cfg(since_274)]
    ("mesanim", config_crc::MESANIM),
    #[cfg(since_274)]
    ("mes", config_crc::MES),
    #[cfg(since_274)]
    ("param", config_crc::PARAM),
    #[cfg(since_274)]
    ("hunt", config_crc::HUNT),
];

const CONFIG_ENTRY_NAMES: &[&str] = &[
    "seq.dat",
    "seq.idx",
    "loc.dat",
    "loc.idx",
    "flo.dat",
    "flo.idx",
    "spotanim.dat",
    "spotanim.idx",
    "obj.dat",
    "obj.idx",
    "npc.dat",
    "npc.idx",
    "idk.dat",
    "idk.idx",
    "varp.dat",
    "varp.idx",
    #[cfg(since_254)]
    "varbit.dat",
    #[cfg(since_254)]
    "varbit.idx",
    #[cfg(since_274)]
    "mesanim.dat",
    #[cfg(since_274)]
    "mesanim.idx",
    #[cfg(since_274)]
    "mes.dat",
    #[cfg(since_274)]
    "mes.idx",
    #[cfg(since_274)]
    "param.dat",
    #[cfg(since_274)]
    "param.idx",
    #[cfg(since_274)]
    "hunt.dat",
    #[cfg(since_274)]
    "hunt.idx",
];

const INTERFACE_ENTRY_NAMES: &[&str] = &["data"];

#[derive(Clone, Copy)]
enum Archive {
    Title,
    Config,
    Interface,
    Media,
    Textures,
    Wordenc,
    Sounds,
    #[cfg(since_244)]
    VersionList,
}

impl Archive {
    #[cfg(rev = "225")]
    fn file_name(self) -> &'static str {
        match self {
            Archive::Title => "title",
            Archive::Config => "config",
            Archive::Interface => "interface",
            Archive::Media => "media",
            Archive::Textures => "textures",
            Archive::Wordenc => "wordenc",
            Archive::Sounds => "sounds",
        }
    }

    #[cfg(since_244)]
    fn idx0_file(self) -> usize {
        match self {
            Archive::Title => 1,
            Archive::Config => 2,
            Archive::Interface => 3,
            Archive::Media => 4,
            Archive::Textures => 6,
            Archive::Wordenc => 7,
            Archive::Sounds => 8,
            Archive::VersionList => 5,
        }
    }
}

struct ArchiveSource {
    #[cfg(rev = "225")]
    dir: PathBuf,
    #[cfg(since_244)]
    cache: Js5Store,
}

impl ArchiveSource {
    #[cfg(rev = "225")]
    fn open(expected_dir: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            dir: expected_dir.to_path_buf(),
        })
    }

    #[cfg(since_244)]
    fn open(expected_dir: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            cache: Js5Store::open(expected_dir, 5)?,
        })
    }

    #[cfg(rev = "225")]
    fn archive(&self, kind: Archive) -> Option<Vec<u8>> {
        let name = kind.file_name();
        std::fs::read(self.dir.join("archives").join(name))
            .or_else(|_| std::fs::read(self.dir.join(name)))
            .ok()
    }

    #[cfg(since_244)]
    fn archive(&self, kind: Archive) -> Option<Vec<u8>> {
        self.cache.read(0, kind.idx0_file(), false)
    }

    #[cfg(since_244)]
    fn cache(&self) -> &Js5Store {
        &self.cache
    }
}

pub fn unpack_all(expected_dir: &Path, output_dir: &Path, pack_dir: &Path) -> anyhow::Result<()> {
    debug!("Unpacking assets...");
    debug!("  expected: {}", expected_dir.display());
    debug!("  output:   {}", output_dir.display());

    std::fs::create_dir_all(output_dir)?;
    std::fs::create_dir_all(pack_dir)?;

    let source = ArchiveSource::open(expected_dir)?;
    let mut crc_report = CrcReport::new();
    let mut leftover = report::LeftoverReport::new();

    #[cfg(since_244)]
    let model_textures = Rc::new(build_model_textures(source.cache()));
    #[cfg(rev = "225")]
    let model_textures: Rc<HashMap<u16, HashSet<u16>>> = Rc::new(HashMap::new());

    // Config must run first - it produces model_categories needed by models.
    let config_jag = prepare_jag(
        "config",
        source.archive(Archive::Config),
        jag_crc::CONFIG,
        &config_known_hashes(),
        &mut crc_report,
        &mut leftover,
    );
    let mut model_categories = HashMap::new();
    if let Some(jag) = &config_jag {
        debug!("Unpacking config...");
        let packs = config::unpack_config(jag, output_dir, pack_dir, model_textures.clone())?;

        for (name, expected) in CONFIG_TYPE_CRCS {
            if let Some(dat) = jag.read(&format!("{name}.dat")) {
                crc_report.config(name, &dat.data, *expected);
            }
        }

        let raw_dir = output_dir.join("_raw").join("config");
        let entries = dump_jag_entries(jag, &raw_dir, CONFIG_ENTRY_NAMES)?;
        debug!("  Dumped {} raw config entries", entries.len());

        std::fs::write(pack_dir.join("config.order"), entries.join("\n") + "\n")?;

        leftover.add_config_leftovers(packs.leftovers, packs.dat_trailing);
        model_categories = packs.model_categories;
    }

    let interface_jag = prepare_jag(
        "interface",
        source.archive(Archive::Interface),
        jag_crc::INTERFACE,
        &interface_known_hashes(),
        &mut crc_report,
        &mut leftover,
    );
    if let Some(jag) = &interface_jag
        && let Some(dat) = jag.read("data")
    {
        crc_report.config("interface", &dat.data, config_crc::INTERFACE);
    }

    let media_jag = prepare_jag(
        "media",
        source.archive(Archive::Media),
        jag_crc::MEDIA,
        &media::known_hashes(),
        &mut crc_report,
        &mut leftover,
    );
    let textures_jag = prepare_jag(
        "textures",
        source.archive(Archive::Textures),
        jag_crc::TEXTURES,
        &texture::known_hashes(),
        &mut crc_report,
        &mut leftover,
    );
    let title_jag = prepare_jag(
        "title",
        source.archive(Archive::Title),
        jag_crc::TITLE,
        &title::known_hashes(),
        &mut crc_report,
        &mut leftover,
    );
    let sounds_jag = prepare_jag(
        "sounds",
        source.archive(Archive::Sounds),
        jag_crc::SOUNDS,
        &sound::known_hashes(),
        &mut crc_report,
        &mut leftover,
    );
    let wordenc_jag = prepare_jag(
        "wordenc",
        source.archive(Archive::Wordenc),
        jag_crc::WORDENC,
        &wordenc::known_hashes(),
        &mut crc_report,
        &mut leftover,
    );

    #[cfg(rev = "225")]
    let (models_jag, songs_dir, maps_dir) = {
        // `models` is a jag archive (under `archives/` in the 225 layout); maps and
        // songs are top-level dirs in the distribution.
        let nested = expected_dir.join("archives").join("models");
        let models_path = if nested.exists() {
            nested
        } else {
            expected_dir.join("models")
        };
        let models_jag = if models_path.exists() {
            prepare_jag(
                "models",
                Some(std::fs::read(&models_path)?),
                jag_crc::MODELS,
                &model::known_hashes(),
                &mut crc_report,
                &mut leftover,
            )
        } else {
            None
        };
        (
            models_jag,
            expected_dir.join("songs"),
            expected_dir.join("maps"),
        )
    };

    std::thread::scope(|s| {
        if let Some(jag) = &interface_jag {
            s.spawn(move || {
                debug!("Unpacking interface...");
                let raw_dir = output_dir.join("_raw").join("interface");
                let entries = dump_jag_entries(jag, &raw_dir, INTERFACE_ENTRY_NAMES).unwrap();
                debug!("  Dumped {} raw interface entries", entries.len());

                let registry = PackRegistry::load(Path::new(PACK_DIR))
                    .expect("load pack registry for interface unpack");
                let src_scripts_dir = Path::new(CONTENT_DIR).join("scripts");
                let out_scripts_dir = output_dir.join("scripts");
                interface::unpack_interface(
                    jag,
                    &src_scripts_dir,
                    &out_scripts_dir,
                    pack_dir,
                    &registry,
                )
                .expect("unpack interface source");
            });
        }

        if let Some(jag) = &media_jag {
            s.spawn(move || {
                debug!("Unpacking media...");
                media::unpack_media(jag, output_dir).unwrap();
            });
        }

        if let Some(jag) = &textures_jag {
            s.spawn(move || {
                debug!("Unpacking textures...");
                texture::unpack_textures(jag, output_dir, pack_dir).unwrap();
            });
        }

        if let Some(jag) = &title_jag {
            s.spawn(move || {
                debug!("Unpacking title...");
                title::unpack_title(jag, output_dir).unwrap();
            });
        }

        if let Some(jag) = &sounds_jag {
            s.spawn(move || {
                debug!("Unpacking sounds...");
                sound::unpack_sounds(jag, output_dir, pack_dir).unwrap();
            });
        }

        if let Some(jag) = &wordenc_jag {
            s.spawn(move || {
                debug!("Unpacking wordenc...");
                wordenc::unpack_wordenc(jag, output_dir).unwrap();
            });
        }

        #[cfg(rev = "225")]
        {
            if let Some(jag) = &models_jag {
                s.spawn(move || {
                    debug!("Unpacking models...");
                    model::unpack_models(jag, output_dir, pack_dir, &model_categories).unwrap();
                });
            }

            if songs_dir.exists() {
                s.spawn(|| {
                    debug!("Unpacking songs...");
                    song::unpack_songs(&songs_dir, output_dir).unwrap();
                });
            }

            if maps_dir.exists() {
                s.spawn(|| {
                    debug!("Unpacking maps...");
                    map::unpack_maps(&maps_dir, output_dir).unwrap();
                });
            }
        }
    });

    #[cfg(since_244)]
    {
        let vl_bytes = source.archive(Archive::VersionList);
        let vl_jag = vl_bytes.as_ref().map(|bytes| {
            crc_report.archive("versionlist", bytes, jag_crc::VERSIONLIST);
            JagFile::from(bytes.clone())
        });
        if let Some(jag) = &vl_jag {
            leftover.scan_jag("versionlist", jag, &versionlist_known_hashes());
        }

        let version_list = vl_jag
            .as_ref()
            .map(VersionList::from_jag)
            .unwrap_or_default();

        if let Some(jag) = &vl_jag {
            VersionListMeta::extract(jag, source.cache()).write(&pack_dir.join("version_list"))?;
            collect_js5_ondemand_crcs(jag, source.cache(), &mut crc_report);
        }

        map::unpack_maps(source.cache(), &version_list, output_dir)?;
        model::unpack_models(source.cache(), output_dir, pack_dir, &model_categories)?;
        model::unpack_anims(source.cache(), output_dir, pack_dir)?;
        song::unpack_midi(source.cache(), &version_list, output_dir, pack_dir)?;

        collect_js5_leftovers(source.cache(), expected_dir, &version_list, &mut leftover);
    }

    crc_report.write(output_dir)?;
    leftover.crack_unknown_names();
    leftover.write(output_dir)?;

    let committed_pack = Path::new(PACK_DIR);
    if committed_pack.exists() && !same_dir(committed_pack, pack_dir) {
        let mut report = PackDiffReport::compare(committed_pack, pack_dir)?;
        let committed_maps = Path::new(CONTENT_DIR).join("maps");
        let unpacked_maps = output_dir.join("maps");
        if committed_maps.exists() && !same_dir(&committed_maps, &unpacked_maps) {
            report.compare_maps(&committed_maps, &unpacked_maps)?;
        }
        report.write(output_dir)?;
    }

    let committed_content = Path::new(CONTENT_DIR);
    if let Some(jag) = &config_jag
        && committed_content.exists()
        && !same_dir(committed_content, output_dir)
    {
        match pack_committed_configs() {
            Ok(packed) => {
                configdiff::ConfigDiffReport::compare(
                    jag,
                    &packed,
                    committed_pack,
                    model_textures.clone(),
                )
                .write(output_dir)?;
            }
            Err(e) => warn!("Config-diff skipped: could not pack committed content: {e:#}"),
        }
    }

    debug!("Unpack complete.");
    Ok(())
}

fn pack_committed_configs() -> anyhow::Result<HashMap<String, PackedFile>> {
    let registry = PackRegistry::load(Path::new(PACK_DIR))?;
    pack_assets(&registry, Path::new(CONTENT_DIR), false)
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

fn prepare_jag(
    name: &'static str,
    bytes: Option<Vec<u8>>,
    expected_crc: Option<i32>,
    known: &[i32],
    crc_report: &mut CrcReport,
    leftover: &mut report::LeftoverReport,
) -> Option<JagFile> {
    let bytes = bytes?;
    crc_report.archive(name, &bytes, expected_crc);
    let jag = JagFile::from(bytes);
    leftover.scan_jag(name, &jag, known);
    Some(jag)
}

fn config_known_hashes() -> Vec<i32> {
    CONFIG_ENTRY_NAMES
        .iter()
        .map(|n| JagFile::hash(n))
        .collect()
}

fn interface_known_hashes() -> Vec<i32> {
    INTERFACE_ENTRY_NAMES
        .iter()
        .map(|n| JagFile::hash(n))
        .collect()
}

#[cfg(since_244)]
fn versionlist_known_hashes() -> Vec<i32> {
    TABLE_NAMES.iter().map(|n| JagFile::hash(n)).collect()
}

#[cfg(since_244)]
fn collect_js5_ondemand_crcs(vl_jag: &JagFile, cache: &Js5Store, crc_report: &mut CrcReport) {
    let tables = [
        (1usize, "model", "model_version", "model_crc"),
        (2, "anim", "anim_version", "anim_crc"),
        (3, "midi", "midi_version", "midi_crc"),
        (4, "map", "map_version", "map_crc"),
    ];
    for (index, label, version_table, crc_table) in tables {
        let versions = read_u16_table(vl_jag, version_table);
        let crcs = read_i32_table(vl_jag, crc_table);
        for (id, &expected) in crcs.iter().enumerate() {
            // Recompute exactly as build_version_list does: getcrc over the stored
            // blob minus its trailing 2-byte version (versionlist::crc_no_version).
            let recomputed = cache
                .read(index, id, false)
                .filter(|d| !d.is_empty())
                .map(|d| rs_io::crc::getcrc(&d, 0, d.len().saturating_sub(2)));
            let version = versions.get(id).copied().unwrap_or(0);
            crc_report.js5_ondemand(label, id, version, expected, recomputed);
        }
    }
}

#[cfg(since_244)]
fn collect_js5_leftovers(
    cache: &Js5Store,
    expected_dir: &Path,
    version_list: &VersionList,
    leftover: &mut report::LeftoverReport,
) {
    for file in 0..cache.count(0) {
        if (1..=8).contains(&file) {
            continue;
        }
        if let Some(blob) = cache.read(0, file, false).filter(|d| !d.is_empty()) {
            leftover.js5_unread(0, file, blob.len(), "idx0 slot not handled");
        }
    }

    let referenced: HashSet<usize> = version_list
        .maps
        .iter()
        .flat_map(|m| [m.land_file as usize, m.loc_file as usize])
        .collect();
    for file in 0..cache.count(4) {
        if referenced.contains(&file) {
            continue;
        }
        if let Some(blob) = cache.read(4, file, false).filter(|d| !d.is_empty()) {
            leftover.js5_unread(
                4,
                file,
                blob.len(),
                "map file not referenced by version list",
            );
        }
    }

    for n in 5..=254usize {
        if expected_dir
            .join(format!("main_file_cache.idx{n}"))
            .exists()
        {
            leftover.extra_index(n);
        }
    }
}

/// A sprite group decoded from the cache: tile size, palette (RGB triplets,
/// entry 0 magenta), and per-frame index buffers (`tile_w * tile_h` each).
pub struct DecodedGroup {
    pub tile_w: usize,
    pub tile_h: usize,
    pub palette: Vec<u8>,
    pub frames: Vec<Vec<u8>>,
}

/// Decode one sprite group's `index.dat` metadata and `.dat` pixel data into
/// per-frame index buffers. Returns `None` if the group is empty or invalid.
pub fn decode_group(index_data: &[u8], dat_data: &[u8]) -> Option<DecodedGroup> {
    let mut dat = Packet::from(dat_data.to_vec());

    let index_pos = dat.g2() as usize;
    if index_pos >= index_data.len() {
        return None;
    }

    let mut idx = Packet::from(index_data[index_pos..].to_vec());

    let tile_w = idx.g2() as usize;
    let tile_h = idx.g2() as usize;
    let palette_len = idx.g1() as usize;

    let mut palette = vec![0xFF, 0x00, 0xFF];
    for _ in 1..palette_len {
        palette.push(idx.g1());
        palette.push(idx.g1());
        palette.push(idx.g1());
    }

    let mut frames: Vec<Vec<u8>> = Vec::new();
    while dat.remaining() > 0 {
        let crop_x = idx.g1() as usize;
        let crop_y = idx.g1() as usize;
        let content_w = idx.g2() as usize;
        let content_h = idx.g2() as usize;
        let pixel_order = idx.g1();

        let mut pixels = vec![0u8; tile_w * tile_h];
        if pixel_order == 0 {
            for y in 0..content_h {
                for x in 0..content_w {
                    pixels[(crop_y + y) * tile_w + crop_x + x] = dat.g1();
                }
            }
        } else {
            for x in 0..content_w {
                for y in 0..content_h {
                    pixels[(crop_y + y) * tile_w + crop_x + x] = dat.g1();
                }
            }
        }
        frames.push(pixels);
    }

    if frames.is_empty() {
        return None;
    }

    Some(DecodedGroup {
        tile_w,
        tile_h,
        palette,
        frames,
    })
}

/// Write one decoded group as an indexed sprite sheet TGA at `path`. The palette
/// goes in the color map, the grid dimensions in the image ID.
pub fn write_group_sheet(path: &Path, g: &DecodedGroup) -> anyhow::Result<()> {
    let (w, h, pixels, palette) = crate::sheet::render(g.tile_w, g.tile_h, &g.palette, &g.frames);
    let id = crate::sheet::image_id(g.tile_w, g.tile_h, g.frames.len());
    crate::tga::write(path, &id, &palette, w as u32, h as u32, &pixels)?;
    Ok(())
}

/// Write a `meta/index.order` sidecar: group keys in `index.dat` order.
pub fn write_index_order(archive_dir: &Path, keys: &[String]) -> anyhow::Result<()> {
    let meta = archive_dir.join("meta");
    std::fs::create_dir_all(&meta)?;
    let body: String = keys.iter().map(|k| format!("{k}\n")).collect();
    std::fs::write(meta.join("index.order"), body)?;
    Ok(())
}

pub fn dump_jag_entries(
    jag: &JagFile,
    output_dir: &Path,
    names: &[&str],
) -> anyhow::Result<Vec<String>> {
    std::fs::create_dir_all(output_dir)?;

    let mut jag_order = Vec::new();
    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        for &name in names {
            if JagFile::hash(name) == hash {
                jag_order.push(name.to_string());
                break;
            }
        }
    }

    for name in &jag_order {
        if let Some(data) = jag.read(name) {
            std::fs::write(output_dir.join(name), &data.data)?;
        }
    }

    std::fs::write(
        output_dir.join("_jag_order.txt"),
        jag_order.join("\n") + "\n",
    )?;
    Ok(jag_order)
}

pub fn pack_jag_from_raw(raw_dir: &Path, compression: JagCompression) -> Vec<u8> {
    let order: Vec<String> = std::fs::read_to_string(raw_dir.join("_jag_order.txt"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let mut jag = JagFile::new();
    for name in &order {
        if let Ok(data) = std::fs::read(raw_dir.join(name)) {
            jag.write(name, data);
        }
    }
    jag.build(compression)
}
