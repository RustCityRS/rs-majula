pub mod config;
pub mod map;
pub mod media;
pub mod model;
pub mod report;
pub mod song;
pub mod sound;
pub mod texture;
pub mod title;
pub mod wordenc;

#[cfg(since_244)]
use crate::version_list::VersionListMeta;
use crate::{config_crc, jag_crc};
use image::{Rgba, RgbaImage};
use rs_io::Packet;
use rs_io::jag::{JagCompression, JagFile};
#[cfg(since_244)]
use rs_io::js5::Js5Store;
use std::collections::HashMap;
use std::path::Path;
#[cfg(rev = "225")]
use std::path::PathBuf;
use tracing::debug;

const CONFIG_TYPE_CRCS: [(&str, i32); 8] = [
    ("seq", config_crc::SEQ),
    ("loc", config_crc::LOC),
    ("flo", config_crc::FLO),
    ("spotanim", config_crc::SPOTANIM),
    ("obj", config_crc::OBJ),
    ("npc", config_crc::NPC),
    ("idk", config_crc::IDK),
    ("varp", config_crc::VARP),
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
    let mut crc_report = report::CrcReport::new();
    let mut leftover = report::LeftoverReport::new();

    // Config must run first - it produces model_categories needed by models.
    let mut model_categories = HashMap::new();
    if let Some(jag) = prepare_jag(
        "config",
        source.archive(Archive::Config),
        jag_crc::CONFIG,
        &config_known_hashes(),
        &mut crc_report,
        &mut leftover,
    ) {
        debug!("Unpacking config...");
        let packs = config::unpack_config(&jag, output_dir, pack_dir)?;

        for (name, expected) in CONFIG_TYPE_CRCS {
            if let Some(dat) = jag.read(&format!("{name}.dat")) {
                crc_report.config(name, &dat.data, expected);
            }
        }

        let raw_dir = output_dir.join("_raw").join("config");
        let entries = dump_jag_entries(&jag, &raw_dir, CONFIG_ENTRY_NAMES)?;
        debug!("  Dumped {} raw config entries", entries.len());

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
            .map(crate::version_list::VersionList::from_jag)
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
    leftover.write(output_dir)?;

    debug!("Unpack complete.");
    Ok(())
}

fn prepare_jag(
    name: &'static str,
    bytes: Option<Vec<u8>>,
    expected_crc: Option<i32>,
    known: &[i32],
    crc_report: &mut report::CrcReport,
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
    crate::version_list::TABLE_NAMES
        .iter()
        .map(|n| JagFile::hash(n))
        .collect()
}

#[cfg(since_244)]
fn collect_js5_ondemand_crcs(
    vl_jag: &JagFile,
    cache: &Js5Store,
    crc_report: &mut report::CrcReport,
) {
    let tables = [
        (1usize, "model", "model_version", "model_crc"),
        (2, "anim", "anim_version", "anim_crc"),
        (3, "midi", "midi_version", "midi_crc"),
        (4, "map", "map_version", "map_crc"),
    ];
    for (index, label, version_table, crc_table) in tables {
        let versions = crate::version_list::read_u16_table(vl_jag, version_table);
        let crcs = crate::version_list::read_i32_table(vl_jag, crc_table);
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
    version_list: &crate::version_list::VersionList,
    leftover: &mut report::LeftoverReport,
) {
    use std::collections::HashSet;

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

pub fn decode_sprite_group(
    index_data: &[u8],
    dat_data: &[u8],
    output_dir: &Path,
) -> anyhow::Result<()> {
    let mut dat = Packet::from(dat_data.to_vec());

    let index_pos = dat.g2() as usize;
    if index_pos >= index_data.len() {
        return Ok(());
    }

    let mut idx = Packet::from(index_data[index_pos..].to_vec());

    let tile_w = idx.g2() as u32;
    let tile_h = idx.g2() as u32;
    let palette_len = idx.g1() as usize;

    let mut palette = vec![0xFF00FF];
    for _ in 1..palette_len {
        let r = idx.g1() as u32;
        let g = idx.g1() as u32;
        let b = idx.g1() as u32;
        palette.push((r << 16) | (g << 8) | b);
    }

    let strip_pixels = palette.len() - 1;
    let strip_rows = if tile_w > 0 {
        (strip_pixels as u32).div_ceil(tile_w)
    } else {
        0
    };

    let mut sprite_index = 0u32;
    while dat.remaining() > 0 {
        let crop_x = idx.g1() as u32;
        let crop_y = idx.g1() as u32;
        let content_w = idx.g2() as u32;
        let content_h = idx.g2() as u32;
        let pixel_order = idx.g1();

        let img_h = if sprite_index == 0 {
            tile_h + strip_rows
        } else {
            tile_h
        };
        let mut img = RgbaImage::new(tile_w, img_h);

        for y in 0..img_h {
            for x in 0..tile_w {
                img.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }

        if content_w > 0 && content_h > 0 {
            let pixel_count = (content_w * content_h) as usize;
            let mut indices = vec![0u8; pixel_count];

            if pixel_order == 0 {
                for p in indices.iter_mut().take(pixel_count) {
                    *p = dat.g1();
                }
            } else {
                for x in 0..content_w as usize {
                    for y in 0..content_h as usize {
                        indices[y * content_w as usize + x] = dat.g1();
                    }
                }
            }

            for y in 0..content_h {
                for x in 0..content_w {
                    let pi = indices[(y * content_w + x) as usize] as usize;
                    let rgb = palette[pi.min(palette.len() - 1)];
                    let r = (rgb >> 16) as u8;
                    let g = (rgb >> 8) as u8;
                    let b = rgb as u8;
                    img.put_pixel(crop_x + x, crop_y + y, Rgba([r, g, b, 255]));
                }
            }
        }

        if sprite_index == 0 && strip_rows > 0 {
            let mut pi = 1;
            for sy in tile_h..tile_h + strip_rows {
                for sx in 0..tile_w {
                    if pi < palette.len() {
                        let rgb = palette[pi];
                        let r = (rgb >> 16) as u8;
                        let g = (rgb >> 8) as u8;
                        let b = rgb as u8;
                        img.put_pixel(sx, sy, Rgba([r, g, b, 254]));
                        pi += 1;
                    }
                }
            }
        }

        let out_path = output_dir.join(format!("{sprite_index}.png"));
        img.save(&out_path)?;
        sprite_index += 1;
    }

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
