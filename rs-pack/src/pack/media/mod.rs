#[cfg(before_647)]
use crate::archive::JagArchive;
#[cfg(since_647)]
use crate::archive::{self, ArchiveVersions};
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
#[cfg(since_647)]
use crate::pack::js5::{PreparedGroup, insert_archive, load_name_hashes};
#[cfg(since_647)]
use crate::pack::names::{AssetNames, archive_name};
#[cfg(before_647)]
use crate::pack::util::{media, palette};
#[cfg(before_647)]
use crate::sheet::Parsed;
#[cfg(since_647)]
use crate::{sheet, tga};
#[cfg(since_647)]
use anyhow::{Context, Result, bail, ensure};
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(before_647)]
use rs_io::jag::JagFile;
#[cfg(since_647)]
use rs_io::js5::reference::FileInfo;
use std::collections::HashMap;
use std::fs;
#[cfg(before_647)]
use std::mem::take;
use std::path::Path;
#[cfg(before_647)]
use tracing::debug;

#[cfg(before_647)]
pub fn pack_media_jag(content_dir: &Path) -> Vec<u8> {
    let sprite_dir = content_dir.join("sprites");
    if !sprite_dir.exists() {
        panic!("Could not find sprites dir");
    }

    let jag_entry_order: Vec<String> =
        fs::read_to_string(sprite_dir.join("meta").join("sprite.order"))
            .expect("Missing sprite.order")
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

    let index_order = media::read_index_order(&sprite_dir);
    let mut index: Vec<u8> = Vec::new();
    let mut dat_map: HashMap<String, Vec<u8>> = HashMap::new();

    let mut groups: Vec<(String, Parsed)> = index_order
        .iter()
        .map(|name| {
            let group = media::read_group(&sprite_dir.join(format!("{name}.tga")));
            (name.clone(), group)
        })
        .collect();
    palette::derive_media(&mut groups);

    for (name, group) in &groups {
        let data = media::emit_group(&mut index, group);
        dat_map.insert(name.clone(), data);
    }

    if dat_map.is_empty() {
        return Vec::new();
    }

    let mut jag = JagFile::new();
    for entry in &jag_entry_order {
        if entry == "index" {
            jag.write("index.dat", take(&mut index));
        } else if let Some(dat) = dat_map.remove(entry.as_str()) {
            jag.write(&format!("{entry}.dat"), dat);
        }
    }

    debug!("Packed sprite groups into media Jag");
    jag.build(JagArchive::Media.compression())
}

#[cfg(since_647)]
pub fn pack_archive(
    store: &mut PackedCache,
    content_dir: &Path,
    pack_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    let names = AssetNames::load(pack_dir)?;
    for &(aid, dir_name) in &[
        (archive::SPRITES, "sprites"),
        (archive::LOADING_SPRITES, "loading_sprites"),
        (archive::LOADING_SPRITES_RAW, "loading_sprites_raw"),
    ] {
        let format = archive::archive_format(aid).unwrap();
        let files = names.files(
            &content_dir.join(dir_name),
            archive_name(aid),
            &["sprite", "jpg"],
        )?;
        let ids: Vec<_> = files.iter().map(|(id, _)| *id).collect();
        let hashes = load_name_hashes(pack_dir, aid, &ids);
        let groups = files
            .par_iter()
            .map(|(gid, path)| {
                let data = encode_sprite_sheet(path).with_context(|| path.display().to_string())?;
                let mut group = PreparedGroup::new(aid, *gid, &data, versions)?;
                group.name_hash = hashes.get(gid).copied().unwrap_or(-1);
                group.files = vec![FileInfo {
                    id: 0,
                    name_hash: None,
                }];
                Ok(group)
            })
            .collect::<Result<Vec<_>>>()?;
        insert_archive(store, aid, format, versions, groups, "from TGA sheets")?;
    }
    Ok(())
}

#[cfg(since_647)]
struct SpriteGroup {
    area_width: u16,
    area_height: u16,
    palette: Vec<u32>,
    raw_palette: Vec<u32>,
    sprites: Vec<SpriteFrame>,
}

#[cfg(since_647)]
struct SpriteFrame {
    width: u16,
    height: u16,
    left_padding: u16,
    top_padding: u16,
    has_alpha: bool,
    row_major: bool,
    indices: Vec<u8>,
    alphas: Option<Vec<u8>>,
}

#[cfg(since_647)]
pub fn encode_sprite_sheet(path: &Path) -> Result<Vec<u8>> {
    if path.extension().is_some_and(|e| e == "jpg") {
        return Ok(fs::read(path)?);
    }
    let text = fs::read_to_string(path)?;
    let mut width = 0u16;
    let mut height = 0u16;
    let mut frames = Vec::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('#') && !s.starts_with("//"))
    {
        let (key, value) = line.split_once('=').context("Expected sprite key=value")?;
        match key.trim() {
            "width" => width = value.trim().parse()?,
            "height" => height = value.trim().parse()?,
            "frame" => {
                let v: Vec<_> = value.split(',').map(str::trim).collect();
                ensure!(
                    v.len() == 6,
                    "Expected frame=x,y,width,height,rows|columns,opaque|alpha"
                );
                ensure!(
                    matches!(v[4], "rows" | "columns") && matches!(v[5], "opaque" | "alpha"),
                    "Invalid sprite flags"
                );
                frames.push(SpriteFrame {
                    left_padding: v[0].parse()?,
                    top_padding: v[1].parse()?,
                    width: v[2].parse()?,
                    height: v[3].parse()?,
                    row_major: v[4] == "columns",
                    has_alpha: v[5] == "alpha",
                    indices: Vec::new(),
                    alphas: None,
                });
            }
            _ => bail!("Unknown sprite property: {key}"),
        }
    }
    ensure!(
        width > 0 && height > 0 && !frames.is_empty() && frames.len() <= u16::MAX as usize,
        "Invalid sprite dimensions"
    );
    let image = tga::read(&path.with_extension("tga"));
    let alpha = frames
        .iter()
        .any(|f| f.has_alpha)
        .then(|| tga::read(&path.with_extension("alpha.tga")));
    let (cols, rows) = sheet::grid(width as usize, height as usize, frames.len());
    ensure!(
        image.width == cols * width as usize && image.height == rows * height as usize,
        "Sprite sheet dimensions do not match metadata"
    );
    ensure!(
        (1..=256).contains(&(image.palette.len() / 3)),
        "Sprite palette exceeds 256 colours"
    );
    if let Some(alpha) = &alpha {
        ensure!(
            alpha.width == image.width && alpha.height == image.height,
            "Alpha sheet size mismatch"
        );
    }
    for (i, frame) in frames.iter_mut().enumerate() {
        ensure!(
            frame.left_padding as usize + frame.width as usize <= width as usize
                && frame.top_padding as usize + frame.height as usize <= height as usize,
            "Sprite crop outside canvas"
        );
        let ox = i % cols * width as usize + frame.left_padding as usize;
        let oy = i / cols * height as usize + frame.top_padding as usize;
        let mut alphas = Vec::new();
        for y in 0..frame.height as usize {
            let start = (oy + y) * image.width + ox;
            frame
                .indices
                .extend_from_slice(&image.pixels[start..start + frame.width as usize]);
            if frame.has_alpha {
                let alpha = alpha.as_ref().unwrap();
                for index in &alpha.pixels[start..start + frame.width as usize] {
                    alphas.push(alpha.palette[*index as usize * 3]);
                }
            }
        }
        ensure!(
            frame
                .indices
                .iter()
                .all(|&i| i as usize * 3 < image.palette.len()),
            "Sprite palette index out of range"
        );
        frame.alphas = frame.has_alpha.then_some(alphas);
    }
    let palette: Vec<u32> = image
        .palette
        .as_chunks::<3>()
        .0
        .iter()
        .map(|c| (c[0] as u32) << 16 | (c[1] as u32) << 8 | c[2] as u32)
        .collect();
    Ok(encode_sprite_group_binary(&SpriteGroup {
        area_width: width,
        area_height: height,
        raw_palette: palette.clone(),
        palette,
        sprites: frames,
    }))
}

#[cfg(since_647)]
fn encode_sprite_group_binary(group: &SpriteGroup) -> Vec<u8> {
    let sprite_count = group.sprites.len();
    let palette = &group.palette;

    let mut pixel_data = Vec::new();
    for sprite in &group.sprites {
        let w = sprite.width as usize;
        let h = sprite.height as usize;
        let pixel_count = w * h;
        let mut flags: u8 = 0;
        if sprite.row_major {
            flags |= 0x01;
        }
        if sprite.has_alpha {
            flags |= 0x02;
        }
        pixel_data.push(flags);

        if sprite.has_alpha {
            let opaque_fallback = vec![0xFFu8; pixel_count];
            let alpha = sprite.alphas.as_deref().unwrap_or(&opaque_fallback);
            if !sprite.row_major {
                pixel_data.extend_from_slice(&sprite.indices[..pixel_count]);
                pixel_data.extend_from_slice(&alpha[..pixel_count]);
            } else {
                for x in 0..w {
                    for y in 0..h {
                        pixel_data.push(sprite.indices[x + y * w]);
                    }
                }
                for x in 0..w {
                    for y in 0..h {
                        pixel_data.push(alpha[x + y * w]);
                    }
                }
            }
        } else if !sprite.row_major {
            pixel_data.extend_from_slice(&sprite.indices[..pixel_count]);
        } else {
            for x in 0..w {
                for y in 0..h {
                    pixel_data.push(sprite.indices[x + y * w]);
                }
            }
        }
    }

    let binary_palette = &group.raw_palette;
    let mut palette_data = Vec::new();
    for &rgb in binary_palette.iter().skip(1) {
        palette_data.push(((rgb >> 16) & 0xFF) as u8);
        palette_data.push(((rgb >> 8) & 0xFF) as u8);
        palette_data.push((rgb & 0xFF) as u8);
    }

    let mut meta = Vec::new();
    meta.extend_from_slice(&group.area_width.to_be_bytes());
    meta.extend_from_slice(&group.area_height.to_be_bytes());
    meta.push((palette.len() - 1) as u8);
    for s in &group.sprites {
        meta.extend_from_slice(&s.left_padding.to_be_bytes());
    }
    for s in &group.sprites {
        meta.extend_from_slice(&s.top_padding.to_be_bytes());
    }
    for s in &group.sprites {
        meta.extend_from_slice(&s.width.to_be_bytes());
    }
    for s in &group.sprites {
        meta.extend_from_slice(&s.height.to_be_bytes());
    }

    let mut out = Vec::with_capacity(pixel_data.len() + palette_data.len() + meta.len() + 2);
    out.extend_from_slice(&pixel_data);
    out.extend_from_slice(&palette_data);
    out.extend_from_slice(&meta);
    out.extend_from_slice(&(sprite_count as u16).to_be_bytes());
    out
}
