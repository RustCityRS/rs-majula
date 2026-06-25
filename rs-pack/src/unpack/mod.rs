pub mod config;
pub mod map;
pub mod media;
pub mod model;
pub mod song;
pub mod sound;
pub mod texture;
pub mod title;
pub mod wordenc;

#[cfg(since_244)]
use crate::version_list::VersionListMeta;
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

    // Config must run first - it produces model_categories needed by models.
    let mut model_categories = HashMap::new();
    if let Some(bytes) = source.archive(Archive::Config) {
        debug!("Unpacking config...");
        let jag = JagFile::from(bytes);
        let packs = config::unpack_config(&jag, output_dir, pack_dir)?;
        model_categories = packs.model_categories;
        let raw_dir = output_dir.join("_raw").join("config");
        let entries = dump_jag_entries(&jag, &raw_dir, CONFIG_ENTRY_NAMES)?;
        debug!("  Dumped {} raw config entries", entries.len());
    }
    let interface_bytes = source.archive(Archive::Interface);
    let media_bytes = source.archive(Archive::Media);
    let textures_bytes = source.archive(Archive::Textures);
    let title_bytes = source.archive(Archive::Title);
    let sounds_bytes = source.archive(Archive::Sounds);
    let wordenc_bytes = source.archive(Archive::Wordenc);

    #[cfg(rev = "225")]
    let (models_path, songs_dir, maps_dir) = {
        // `models` is a jag archive (under `archives/` in the 225 layout); maps and
        // songs are top-level dirs in the distribution.
        let nested = expected_dir.join("archives").join("models");
        let models_path = if nested.exists() {
            nested
        } else {
            expected_dir.join("models")
        };
        (
            models_path,
            expected_dir.join("songs"),
            expected_dir.join("maps"),
        )
    };

    std::thread::scope(|s| {
        if let Some(bytes) = interface_bytes {
            s.spawn(move || {
                debug!("Unpacking interface...");
                let jag = JagFile::from(bytes);
                let raw_dir = output_dir.join("_raw").join("interface");
                let entries = dump_jag_entries(&jag, &raw_dir, INTERFACE_ENTRY_NAMES).unwrap();
                debug!("  Dumped {} raw interface entries", entries.len());
            });
        }

        if let Some(bytes) = media_bytes {
            s.spawn(move || {
                debug!("Unpacking media...");
                media::unpack_media(&JagFile::from(bytes), output_dir).unwrap();
            });
        }

        if let Some(bytes) = textures_bytes {
            s.spawn(move || {
                debug!("Unpacking textures...");
                texture::unpack_textures(&JagFile::from(bytes), output_dir, pack_dir).unwrap();
            });
        }

        if let Some(bytes) = title_bytes {
            s.spawn(move || {
                debug!("Unpacking title...");
                title::unpack_title(&JagFile::from(bytes), output_dir).unwrap();
            });
        }

        if let Some(bytes) = sounds_bytes {
            s.spawn(move || {
                debug!("Unpacking sounds...");
                sound::unpack_sounds(&JagFile::from(bytes), output_dir, pack_dir).unwrap();
            });
        }

        if let Some(bytes) = wordenc_bytes {
            s.spawn(move || {
                debug!("Unpacking wordenc...");
                wordenc::unpack_wordenc(&JagFile::from(bytes), output_dir).unwrap();
            });
        }

        #[cfg(rev = "225")]
        {
            if models_path.exists() {
                s.spawn(|| {
                    debug!("Unpacking models...");
                    let jag = JagFile::from(std::fs::read(&models_path).unwrap());
                    model::unpack_models(&jag, output_dir, pack_dir, &model_categories).unwrap();
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
        let vl_jag = source.archive(Archive::VersionList).map(JagFile::from);
        let version_list = vl_jag
            .as_ref()
            .map(crate::version_list::VersionList::from_jag)
            .unwrap_or_default();

        if let Some(jag) = &vl_jag {
            VersionListMeta::extract(jag, source.cache()).write(&pack_dir.join("version_list"))?;
        }

        map::unpack_maps(source.cache(), &version_list, output_dir)?;
        model::unpack_models(source.cache(), output_dir, pack_dir, &model_categories)?;
        model::unpack_anims(source.cache(), output_dir, pack_dir)?;
        song::unpack_midi(source.cache(), &version_list, output_dir, pack_dir)?;
    }

    debug!("Unpack complete.");
    Ok(())
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
