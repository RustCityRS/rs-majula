#[cfg(all(since_244, before_647))]
use crate::archive;
#[cfg(since_647)]
use crate::archive::CONFIG_UNKNOWN_GROUPS;
#[cfg(before_647)]
use crate::archive::{JagArchive, JagSource};
#[cfg(since_647)]
use crate::discover::discover_symbols;
#[cfg(since_647)]
use crate::pack::pack::FileCache;
#[cfg(before_647)]
use crate::pack::pack::pack_assets;
#[cfg(since_647)]
use crate::pack::pack_registry::PackFile;
#[cfg(before_647)]
use crate::pack::pack_registry::{PackRegistry, PackedFile};
#[cfg(before_647)]
use crate::pack::util::palette::detect_scan_cols;
#[cfg(all(since_244, before_647))]
use crate::versionlist::{
    TABLE_NAMES, VersionList, VersionListMeta, read_i32_table, read_u16_table,
};
#[cfg(before_647)]
use crate::{CONFIG_TYPE_CRCS, CONTENT_DIR, PACK_DIR, config_crc, sheet, tga};
#[cfg(since_647)]
use crate::{CONFIG_TYPE_CRCS, PACK_DIR};
#[cfg(since_647)]
use anyhow::{Result, bail};
#[cfg(since_647)]
use config::generic;
#[cfg(before_647)]
use configdiff::ConfigDiffReport;
#[cfg(all(since_244, before_647))]
use model::build_model_textures;
#[cfg(before_647)]
use report::{CrcReport, LeftoverReport, PackDiffReport};
#[cfg(since_647)]
use report::{StepResult, step, write_report};
#[cfg(before_647)]
use rs_io::Packet;
#[cfg(all(since_244, before_647))]
use rs_io::crc::getcrc;
#[cfg(before_647)]
use rs_io::jag::{JagCompression, JagFile};
#[cfg(since_244)]
use rs_io::js5::Js5Store;
#[cfg(before_647)]
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
#[cfg(before_647)]
use std::rc::Rc;
#[cfg(before_647)]
use std::thread;
#[cfg(since_647)]
use tracing::info;
#[cfg(before_647)]
use tracing::{debug, warn};

#[cfg(since_647)]
pub mod binary_assets;
pub mod config;
#[cfg(before_647)]
pub mod configdiff;
#[cfg(since_647)]
pub mod cs2;
pub mod interface;
pub mod map;
pub mod media;
pub mod model;
#[cfg(before_647)]
mod namecrack;
#[cfg(since_647)]
pub mod names;
pub mod report;
#[cfg(since_647)]
pub mod shaders;
pub mod song;
#[cfg(before_647)]
pub mod sound;
pub mod texture;
#[cfg(before_647)]
pub mod title;
#[cfg(before_647)]
pub mod wordenc;

#[cfg(before_647)]
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

#[cfg(before_647)]
const INTERFACE_ENTRY_NAMES: &[&str] = &["data"];

pub fn unpack_all(expected_dir: &Path, output_dir: &Path, pack_dir: &Path) -> anyhow::Result<()> {
    #[cfg(since_647)]
    {
        unpack_cache(expected_dir, output_dir, pack_dir)
    }
    #[cfg(before_647)]
    {
        debug!("Unpacking assets...");
        debug!("  expected: {}", expected_dir.display());
        debug!("  output:   {}", output_dir.display());

        fs::create_dir_all(output_dir)?;
        fs::create_dir_all(pack_dir)?;

        let mut source = JagSource::open(expected_dir)?;
        let mut crc_report = CrcReport::new();
        let mut leftover = LeftoverReport::new();

        #[cfg(since_244)]
        let model_textures = Rc::new(build_model_textures(source.cache()));
        #[cfg(rev = "225")]
        let model_textures: Rc<HashMap<u16, HashSet<u16>>> = Rc::new(HashMap::new());

        // Config must run first - it produces model_categories needed by models.
        let config_jag = prepare_jag(
            JagArchive::Config,
            source.archive(JagArchive::Config),
            &config_known_hashes(),
            &mut crc_report,
            &mut leftover,
        );
        let mut model_categories = HashMap::new();
        if let Some(jag) = &config_jag {
            debug!("Unpacking config...");
            let config_dir = output_dir.join("scripts/globalscript/_unpack");
            fs::create_dir_all(&config_dir)?;
            let packs = config::unpack_config(jag, &config_dir, pack_dir, model_textures.clone())?;

            for (name, expected) in CONFIG_TYPE_CRCS {
                if let Some(dat) = jag.read(&format!("{name}.dat")) {
                    crc_report.config(name, &dat.data, *expected);
                }
            }

            let raw_dir = output_dir.join("_raw").join("config");
            let entries = dump_jag_entries(jag, &raw_dir, CONFIG_ENTRY_NAMES)?;
            debug!("  Dumped {} raw config entries", entries.len());

            fs::write(pack_dir.join("config.order"), entries.join("\n") + "\n")?;

            leftover.add_config_leftovers(packs.leftovers, packs.dat_trailing);
            model_categories = packs.model_categories;
        }

        let interface_jag = prepare_jag(
            JagArchive::Interface,
            source.archive(JagArchive::Interface),
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
            JagArchive::Media,
            source.archive(JagArchive::Media),
            &media::known_hashes(),
            &mut crc_report,
            &mut leftover,
        );
        let textures_jag = prepare_jag(
            JagArchive::Textures,
            source.archive(JagArchive::Textures),
            &texture::known_hashes(),
            &mut crc_report,
            &mut leftover,
        );
        let title_jag = prepare_jag(
            JagArchive::Title,
            source.archive(JagArchive::Title),
            &title::known_hashes(),
            &mut crc_report,
            &mut leftover,
        );
        let sounds_jag = prepare_jag(
            JagArchive::Sounds,
            source.archive(JagArchive::Sounds),
            &sound::known_hashes(),
            &mut crc_report,
            &mut leftover,
        );
        let wordenc_jag = prepare_jag(
            JagArchive::Wordenc,
            source.archive(JagArchive::Wordenc),
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
                    JagArchive::Models,
                    Some(fs::read(&models_path)?),
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

        thread::scope(|s| {
            if let Some(jag) = &interface_jag {
                s.spawn(move || {
                    debug!("Unpacking interface...");
                    let raw_dir = output_dir.join("_raw").join("interface");
                    let entries = dump_jag_entries(jag, &raw_dir, INTERFACE_ENTRY_NAMES).unwrap();
                    debug!("  Dumped {} raw interface entries", entries.len());

                    let registry = PackRegistry::load(Path::new(PACK_DIR))
                        .expect("load pack registry for interface unpack");
                    let src_scripts_dir = Path::new(CONTENT_DIR).join("scripts/runescript");
                    let out_scripts_dir = output_dir.join("scripts/runescript");
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
            let vl_bytes = source.archive(JagArchive::VersionList);
            let vl_jag = vl_bytes.as_ref().map(|bytes| {
                crc_report.archive("versionlist", bytes, JagArchive::VersionList.expected_crc());
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
                VersionListMeta::extract(jag, source.cache())
                    .write(&pack_dir.join("version_list"))?;
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
                    ConfigDiffReport::compare(jag, &packed, committed_pack, model_textures.clone())
                        .write(output_dir)?;
                }
                Err(e) => warn!("Config-diff skipped: could not pack committed content: {e:#}"),
            }
        }

        debug!("Unpack complete.");
        Ok(())
    }
}

#[cfg(before_647)]
fn pack_committed_configs() -> anyhow::Result<HashMap<String, PackedFile>> {
    let registry = PackRegistry::load(Path::new(PACK_DIR))?;
    pack_assets(&registry, &Path::new(CONTENT_DIR).join("scripts"), false)
}

#[cfg(before_647)]
fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => a == b,
    }
}

#[cfg(before_647)]
fn prepare_jag(
    kind: JagArchive,
    bytes: Option<Vec<u8>>,
    known: &[i32],
    crc_report: &mut CrcReport,
    leftover: &mut LeftoverReport,
) -> Option<JagFile> {
    let name = kind.name();
    let bytes = bytes?;
    crc_report.archive(name, &bytes, kind.expected_crc());
    let jag = JagFile::from(bytes);
    leftover.scan_jag(name, &jag, known);
    Some(jag)
}

#[cfg(before_647)]
fn config_known_hashes() -> Vec<i32> {
    CONFIG_ENTRY_NAMES
        .iter()
        .map(|n| JagFile::hash(n))
        .collect()
}

#[cfg(before_647)]
fn interface_known_hashes() -> Vec<i32> {
    INTERFACE_ENTRY_NAMES
        .iter()
        .map(|n| JagFile::hash(n))
        .collect()
}

#[cfg(all(since_244, before_647))]
fn versionlist_known_hashes() -> Vec<i32> {
    TABLE_NAMES.iter().map(|n| JagFile::hash(n)).collect()
}

#[cfg(all(since_244, before_647))]
fn collect_js5_ondemand_crcs(vl_jag: &JagFile, cache: &mut Js5Store, crc_report: &mut CrcReport) {
    let tables = [
        (1u8, "model", "model_version", "model_crc"),
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
                .read_raw_group_bytes(index, id as u32)
                .ok()
                .flatten()
                .filter(|d| !d.is_empty())
                .map(|d| getcrc(&d, 0, d.len().saturating_sub(2)));
            let version = versions.get(id).copied().unwrap_or(0);
            crc_report.js5_ondemand(label, id, version, expected, recomputed);
        }
    }
}

#[cfg(all(since_244, before_647))]
fn collect_js5_leftovers(
    cache: &mut Js5Store,
    expected_dir: &Path,
    version_list: &VersionList,
    leftover: &mut LeftoverReport,
) {
    for file in 0..cache.group_count(archive::JAG).unwrap_or(0) as usize {
        if (1..=8).contains(&file) {
            continue;
        }
        if let Some(blob) = cache
            .read_raw_group_bytes(archive::JAG, file as u32)
            .ok()
            .flatten()
            .filter(|d| !d.is_empty())
        {
            leftover.js5_unread(0, file, blob.len(), "idx0 slot not handled");
        }
    }

    let referenced: HashSet<usize> = version_list
        .maps
        .iter()
        .flat_map(|m| [m.land_file as usize, m.loc_file as usize])
        .collect();
    for file in 0..cache.group_count(archive::MAPS).unwrap_or(0) as usize {
        if referenced.contains(&file) {
            continue;
        }
        if let Some(blob) = cache
            .read_raw_group_bytes(archive::MAPS, file as u32)
            .ok()
            .flatten()
            .filter(|d| !d.is_empty())
        {
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

#[cfg(before_647)]
/// A sprite group decoded from the cache: tile size, palette (RGB triplets,
/// entry 0 magenta), and per-frame index buffers (`tile_w * tile_h` each).
pub struct DecodedGroup {
    pub tile_w: usize,
    pub tile_h: usize,
    pub palette: Vec<u8>,
    pub frames: Vec<Vec<u8>>,
    /// Palette scan-grid column count when the default layout doesn't reproduce
    /// the cache palette (stamped into the TGA image ID; see
    /// `pack::util::palette::detect_scan_cols`).
    pub scan_cols: Option<usize>,
}

#[cfg(before_647)]
impl DecodedGroup {
    /// Check the cache palette against the default scan grid and record the
    /// column count to stamp when a bespoke layout is needed.
    pub fn detect_scan_cols(&mut self, name: &str) {
        self.scan_cols =
            detect_scan_cols(name, self.tile_w, self.tile_h, &self.palette, &self.frames);
    }
}

#[cfg(before_647)]
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
        scan_cols: None,
    })
}

#[cfg(before_647)]
/// Write one decoded group as an indexed sprite sheet TGA at `path`. The palette
/// goes in the color map, the grid dimensions (and any scan-grid column stamp)
/// in the image ID.
pub fn write_group_sheet(path: &Path, g: &DecodedGroup) -> anyhow::Result<()> {
    let (w, h, pixels, palette) = sheet::render(g.tile_w, g.tile_h, &g.palette, &g.frames);
    let id = sheet::image_id(g.tile_w, g.tile_h, g.frames.len(), g.scan_cols);
    tga::write(path, &id, &palette, w as u32, h as u32, &pixels)?;
    Ok(())
}

#[cfg(before_647)]
/// Write a `meta/index.order` sidecar: group keys in `index.dat` order.
pub fn write_index_order(archive_dir: &Path, keys: &[String]) -> anyhow::Result<()> {
    let meta = archive_dir.join("meta");
    fs::create_dir_all(&meta)?;
    let body: String = keys.iter().map(|k| format!("{k}\n")).collect();
    fs::write(meta.join("index.order"), body)?;
    Ok(())
}

#[cfg(before_647)]
pub fn dump_jag_entries(
    jag: &JagFile,
    output_dir: &Path,
    names: &[&str],
) -> anyhow::Result<Vec<String>> {
    fs::create_dir_all(output_dir)?;

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
            fs::write(output_dir.join(name), &data.data)?;
        }
    }

    fs::write(
        output_dir.join("_jag_order.txt"),
        jag_order.join("\n") + "\n",
    )?;
    Ok(jag_order)
}

#[cfg(before_647)]
pub fn pack_jag_from_raw(raw_dir: &Path, compression: JagCompression) -> Vec<u8> {
    let order: Vec<String> = fs::read_to_string(raw_dir.join("_jag_order.txt"))
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect();

    let mut jag = JagFile::new();
    for name in &order {
        if let Ok(data) = fs::read(raw_dir.join(name)) {
            jag.write(name, data);
        }
    }
    jag.build(compression)
}

#[cfg(since_647)]
fn prepare_pack(pack_dir: &Path) -> Result<()> {
    fs::create_dir_all(pack_dir)?;
    for entry in fs::read_dir(PACK_DIR)? {
        let entry = entry?;
        let path = entry.path();
        let target = pack_dir.join(entry.file_name());
        if path.extension().is_some_and(|ext| ext == "pack") && !target.exists() {
            fs::copy(path, target)?;
        }
    }
    Ok(())
}

#[cfg(since_647)]
fn unpack_cache(expected_dir: &Path, output_dir: &Path, pack_dir: &Path) -> Result<()> {
    let configs = FileCache::new(&output_dir.join("scripts"));
    if let Some(path) = CONFIG_TYPE_CRCS
        .iter()
        .map(|(kind, _)| *kind)
        .chain(FileCache::TEXT_EXTS.iter().copied())
        .find_map(|kind| configs.collect(kind).first())
    {
        bail!(
            "Output already contains configs at {}. Unpack into an empty directory and reconcile changes with the existing definitions.",
            path.display()
        );
    }
    fs::create_dir_all(output_dir)?;
    prepare_pack(pack_dir)?;
    let scripts_dir = output_dir.join("scripts/globalscript");
    fs::create_dir_all(scripts_dir.join("_unpack"))?;
    let mut results: Vec<StepResult> = Vec::new();
    info!("Discovering pack registries from cache...");
    let report = discover_symbols(expected_dir, pack_dir, false)?;
    info!("  {} registry types processed", report.types.len());
    step(&mut results, "archive metadata", || {
        let store = Js5Store::open(expected_dir)?;
        asset::unpack_metadata(&store, pack_dir, output_dir).map(|n| format!("{n} archives"))
    });
    unpack_configs(&mut results, expected_dir, pack_dir, &scripts_dir);
    step(&mut results, "clientscripts", || {
        cs2::unpack_cs2_scripts(expected_dir, output_dir, pack_dir).map(|n| format!("{n} scripts"))
    });
    step(&mut results, "maps", || {
        let mut store = Js5Store::open(expected_dir)?;
        map::unpack_maps(&mut store, expected_dir, output_dir, pack_dir)
            .map(|n| format!("{n} map groups"))
    });
    unpack_assets(&mut results, expected_dir, pack_dir, output_dir);
    write_report(&results, &output_dir.join("unpack_report.txt"))
}

#[cfg(since_647)]
fn unpack_configs(
    results: &mut Vec<StepResult>,
    cache_dir: &Path,
    pack_dir: &Path,
    scripts_dir: &Path,
) {
    let config_out = scripts_dir.join("_unpack");

    if let Err(error) = config::unpack_config(cache_dir, &config_out, pack_dir, results) {
        step(results, "configs", || Err(error));
    }

    // CacheType structs. Known types get named opcodes; unknown groups get
    // raw hex dumps.
    step(results, "generic config groups", || {
        let mut store = Js5Store::open(cache_dir)?;
        let seq = PackFile::load(&pack_dir.join("seq.pack"))?;
        for &(group_id, type_name) in generic::KNOWN_CONFIG_GROUPS {
            info!("  Unpacking {type_name} (group {group_id})...");
            generic::unpack_config_group(&mut store, group_id, type_name, &config_out, &seq)?;
        }
        for &group_id in CONFIG_UNKNOWN_GROUPS {
            info!("  Unpacking config{group_id} (raw)...");
            generic::unpack_raw_config_group(&mut store, group_id, &config_out)?;
        }
        Ok(String::new())
    });
}

#[cfg(since_647)]
fn unpack_assets(
    results: &mut Vec<StepResult>,
    cache_dir: &Path,
    pack_dir: &Path,
    output_dir: &Path,
) {
    step(results, "animations", || {
        let mut store = Js5Store::open(cache_dir)?;
        model::unpack_anims(&mut store, pack_dir, output_dir).map(|n| format!("{n} frames"))
    });
    step(results, "interfaces", || {
        let mut store = Js5Store::open(cache_dir)?;
        interface::unpack_interfaces(&mut store, output_dir, pack_dir)
            .map(|n| format!("{n} groups"))
    });
    step(results, "models", || {
        model::unpack_models(cache_dir, pack_dir, output_dir).map(|n| format!("{n} models"))
    });
    step(results, "sprites", || {
        let mut store = Js5Store::open(cache_dir)?;
        let stats = media::unpack_all_sprites(&mut store, output_dir, pack_dir)?;
        Ok(format!(
            "{} groups, {} sprites",
            stats.groups, stats.sprites
        ))
    });
    step(results, "textures", || {
        let mut store = Js5Store::open(cache_dir)?;
        texture::unpack_texture_text(&mut store, output_dir, pack_dir)
            .map(|n| format!("{n} groups"))
    });
    step(results, "music", || {
        let mut store = Js5Store::open(cache_dir)?;
        song::unpack_songs(&mut store, pack_dir, output_dir)
            .map(|n| format!("{n} songs and jingles"))
    });
    for aid in [1, 4, 10, 13, 15, 23, 24, 25, 26, 27, 28, 29, 33] {
        let format = crate::archive::archive_format(aid).unwrap();
        step(results, format.content_dir, || {
            let mut store = Js5Store::open(cache_dir)?;
            asset::unpack_archive(&mut store, aid, pack_dir, output_dir)
                .map(|n| format!("{n} groups"))
        });
    }
    step(results, "shaders", || {
        let mut store = Js5Store::open(cache_dir)?;
        shaders::unpack_shaders(&mut store, output_dir, pack_dir).map(|n| format!("{n} groups"))
    });
    step(results, "Vorbis", || {
        let mut store = Js5Store::open(cache_dir)?;
        crate::pack::sound::vorbis::unpack_archive(&mut store, pack_dir, output_dir)
            .map(|n| format!("{n} groups"))
    });
    step(results, "native libraries", || {
        let mut store = Js5Store::open(cache_dir)?;
        binary_assets::unpack_libraries(&mut store, output_dir, pack_dir)
            .map(|n| format!("{n} groups"))
    });
}
#[cfg(since_647)]
pub mod asset;
