#[cfg(since_647)]
use crate::config_blob::ConfigBlob;
#[cfg(since_647)]
use crate::pack::js5::{PreparedGroup, load_name_hashes, stamp_path, write_archive};
#[cfg(before_647)]
use crate::pack::util::{media, palette};
#[cfg(before_647)]
use crate::sheet::Parsed;
#[cfg(since_647)]
use crate::startup::{is_up_to_date, newest_mtime, write_stamp};
#[cfg(since_647)]
use anyhow::{Context, Result};
#[cfg(since_647)]
use rayon::prelude::*;
#[cfg(since_647)]
use rs_dat2::CacheStore;
#[cfg(since_647)]
use rs_dat2::archive::{self, ArchiveVersions};
#[cfg(since_647)]
use rs_dat2::reference::FileInfo;
#[cfg(before_647)]
use rs_io::jag::{JagCompression, JagFile};
use std::collections::HashMap;
#[cfg(since_647)]
use std::collections::HashSet;
use std::fs;
#[cfg(since_647)]
use std::io::BufReader;
#[cfg(before_647)]
use std::mem::take;
use std::path::Path;
#[cfg(since_647)]
use std::path::PathBuf;
#[cfg(since_647)]
use std::str::from_utf8;
#[cfg(before_647)]
use tracing::debug;
#[cfg(since_647)]
use tracing::info;

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
    jag.build(JagCompression::PerFile)
}

#[cfg(since_647)]
pub fn pack_archive(
    store: &mut CacheStore,
    content_dir: &Path,
    output_dir: &Path,
    pack_dir: &Path,
    client_data_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
    force_rebuild: bool,
) -> Result<()> {
    for &(aid, dir_name) in &[
        (archive::SPRITES, "sprites"),
        (archive::LOADING_SPRITES, "loading_sprites"),
    ] {
        let format = archive::archive_format(aid).unwrap();
        let unpack_dir = content_dir.join(dir_name).join("_unpack");

        if !unpack_dir.is_dir() {
            info!(
                "  archive {:>2} ({}) -- no _unpack dir, skipping",
                aid, dir_name
            );
            continue;
        }

        let sprite_stamp = stamp_path(output_dir, aid);
        let sprite_input = newest_mtime(&unpack_dir, &[]);
        if !force_rebuild && is_up_to_date(&sprite_stamp, sprite_input) {
            info!(
                "  archive {:>2} ({}) -- up to date, skipping",
                aid, dir_name
            );
            continue;
        }

        let mut group_ids: Vec<u32> = Vec::new();
        if let Ok(entries) = fs::read_dir(&unpack_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if let Ok(gid) = name.parse::<u32>() {
                            group_ids.push(gid);
                        }
                    }
                }
            }
        }
        group_ids.sort();

        if group_ids.is_empty() {
            info!(
                "  archive {:>2} ({}) -- no sprite groups found, skipping",
                aid, dir_name
            );
            continue;
        }

        let name_hashes: HashMap<u32, i32> = if format.named {
            load_name_hashes(pack_dir, aid, &group_ids)
        } else {
            HashMap::new()
        };

        let encoded: Vec<(u32, Vec<u8>)> = group_ids
            .par_iter()
            .filter_map(|&gid| {
                let group_dir = unpack_dir.join(gid.to_string());
                match encode_sprite_group_from_dir(&group_dir) {
                    Ok(Some(b)) => Some((gid, b)),
                    Ok(None) => None,
                    Err(e) => {
                        tracing::warn!("sprite group {}: encode failed ({}), skipping", gid, e);
                        None
                    }
                }
            })
            .collect();

        if aid == archive::SPRITES {
            let mut blob = ConfigBlob::new();
            for (gid, data) in &encoded {
                blob.add(&gid.to_string(), data.clone());
            }
            blob.save(&client_data_dir.join("media"))?;
        }

        let prepared: Vec<PreparedGroup> = encoded
            .par_iter()
            .filter_map(|(gid, data)| {
                let mut group = PreparedGroup::new(aid, *gid, data, versions).ok()?;
                group.name_hash = name_hashes.get(gid).copied().unwrap_or(-1);
                group.files = vec![FileInfo {
                    id: 0,
                    name_hash: None,
                }];
                Some(group)
            })
            .collect();

        let blob_note = if aid == archive::SPRITES {
            "PNG encode → media blob"
        } else {
            "PNG encode"
        };
        write_archive(store, aid, format, versions, prepared, blob_note)?;
        write_stamp(&sprite_stamp)?;
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

/// Encode indexed PNG sprites or a raw JPEG group.
#[cfg(since_647)]
pub fn encode_sprite_group_from_dir(group_dir: &Path) -> Result<Option<Vec<u8>>> {
    let jpg_path = group_dir.join("0.jpg");
    if jpg_path.exists() {
        return Ok(Some(fs::read(&jpg_path)?));
    }
    match read_sprite_group(group_dir)? {
        Some(group) => Ok(Some(encode_sprite_group_binary(&group))),
        None => Ok(None),
    }
}

#[cfg(since_647)]
fn read_sprite_group(group_dir: &Path) -> Result<Option<SpriteGroup>> {
    let mut png_files: Vec<(u32, PathBuf)> = Vec::new();
    for entry in fs::read_dir(group_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("png") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if let Ok(id) = stem.parse::<u32>() {
                    png_files.push((id, path));
                }
            }
        }
    }
    if png_files.is_empty() {
        return Ok(None);
    }
    png_files.sort_by_key(|(id, _)| *id);

    let (first_rgba, first_meta) = load_rgba_png_with_meta(&png_files[0].1)?;
    let img_w = first_rgba.width;
    let img_h = first_rgba.height;

    let strip_rows = if first_meta.strip_rows > 0 {
        first_meta.strip_rows
    } else {
        let mut sr = 0u32;
        for y in (0..img_h).rev() {
            let row_start = (y * img_w) as usize * 4;
            let has_sentinel =
                (0..img_w as usize).any(|x| first_rgba.data[row_start + x * 4 + 3] == 254);
            if has_sentinel {
                sr += 1;
            } else {
                break;
            }
        }
        sr
    };

    let area_width = img_w as u16;
    let area_height = (img_h - strip_rows) as u16;

    let (palette, raw_palette) = if let Some(ref hex) = first_meta.palette_hex {
        let mut raw: Vec<u32> = vec![0];
        let mut display: Vec<u32> = vec![0];
        for chunk in hex.as_bytes().chunks(6) {
            if chunk.len() == 6 {
                let s = from_utf8(chunk).unwrap_or("000000");
                let rgb = u32::from_str_radix(s, 16).unwrap_or(0);
                raw.push(rgb);
                display.push(if rgb == 0 { 1 } else { rgb });
            }
        }
        (display, raw)
    } else if strip_rows > 0 {
        let mut pal: Vec<u32> = vec![0];
        for row in 0..strip_rows {
            let y = (img_h - strip_rows + row) as usize;
            for x in 0..img_w as usize {
                let idx = (y * img_w as usize + x) * 4;
                if first_rgba.data[idx + 3] == 254 {
                    let r = first_rgba.data[idx] as u32;
                    let g = first_rgba.data[idx + 1] as u32;
                    let b = first_rgba.data[idx + 2] as u32;
                    pal.push((r << 16) | (g << 8) | b);
                }
            }
        }
        (pal.clone(), pal)
    } else {
        let mut seen = HashSet::new();
        seen.insert(0u32);
        let mut pal: Vec<u32> = vec![0];
        for (_, path) in &png_files {
            let img = load_rgba_png(path)?;
            for i in 0..(img.width * img.height) as usize {
                let a = img.data[i * 4 + 3];
                if a == 0 {
                    continue;
                }
                let r = img.data[i * 4] as u32;
                let g = img.data[i * 4 + 1] as u32;
                let b = img.data[i * 4 + 2] as u32;
                let rgb = (r << 16) | (g << 8) | b;
                if seen.insert(rgb) {
                    pal.push(rgb);
                }
            }
        }
        (pal.clone(), pal)
    };

    let color_map: HashMap<u32, u8> = palette
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, &c)| (c, i as u8))
        .collect();

    let mut sprites = Vec::with_capacity(png_files.len());
    for (file_idx, (_, path)) in png_files.iter().enumerate() {
        let (img, sprite_meta) = if file_idx == 0 {
            (
                first_rgba.clone(),
                PngMeta {
                    strip_rows: first_meta.strip_rows,
                    has_alpha: first_meta.has_alpha,
                    row_major: first_meta.row_major,
                    crop: first_meta.crop,
                    palette_hex: None,
                },
            )
        } else {
            load_rgba_png_with_meta(path)?
        };
        let meta_has_alpha = sprite_meta.has_alpha;
        let meta_row_major = sprite_meta.row_major;

        let aw = area_width as usize;
        let ah = area_height as usize;

        let has_real_alpha = if let Some(ha) = meta_has_alpha {
            ha
        } else {
            let mut detected = false;
            for y in 0..ah {
                for x in 0..aw {
                    let idx = (y * img.width as usize + x) * 4;
                    let a = img.data[idx + 3];
                    if a == 254 {
                        continue;
                    }
                    if a > 0 && a < 255 {
                        detected = true;
                        break;
                    }
                    if a == 0 {
                        let r = img.data[idx] as u32;
                        let g = img.data[idx + 1] as u32;
                        let b = img.data[idx + 2] as u32;
                        if (r << 16) | (g << 8) | b != 0 {
                            detected = true;
                            break;
                        }
                    }
                }
                if detected {
                    break;
                }
            }
            detected
        };

        let (left_pad, top_pad, content_w, content_h) = if let Some(crop) = sprite_meta.crop {
            crop
        } else {
            let mut min_x = aw;
            let mut min_y = ah;
            let mut max_x = 0usize;
            let mut max_y = 0usize;
            let mut has_content = false;

            for y in 0..ah {
                for x in 0..aw {
                    let idx = (y * img.width as usize + x) * 4;
                    let a = img.data[idx + 3];
                    if a > 0 && a != 254 {
                        has_content = true;
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x);
                        max_y = max_y.max(y);
                    }
                }
            }

            if has_content {
                (
                    min_x as u16,
                    min_y as u16,
                    (max_x - min_x + 1) as u16,
                    (max_y - min_y + 1) as u16,
                )
            } else {
                (0u16, 0u16, 0u16, 0u16)
            }
        };

        let min_x = left_pad as usize;
        let min_y = top_pad as usize;
        let cw = content_w as usize;
        let ch = content_h as usize;
        let pixel_count = cw * ch;

        let mut indices = vec![0u8; pixel_count];
        let mut alphas = vec![0u8; pixel_count];

        for y in 0..ch {
            for x in 0..cw {
                let src_x = min_x + x;
                let src_y = min_y + y;
                let src_idx = (src_y * img.width as usize + src_x) * 4;
                let r = img.data[src_idx] as u32;
                let g = img.data[src_idx + 1] as u32;
                let b = img.data[src_idx + 2] as u32;
                let a = img.data[src_idx + 3];

                alphas[y * cw + x] = a;

                if has_real_alpha && a == 0 {
                    let rgb = (r << 16) | (g << 8) | b;
                    indices[y * cw + x] = if rgb == 0 {
                        0
                    } else {
                        color_map.get(&rgb).copied().unwrap_or(0)
                    };
                } else if has_real_alpha {
                    let rgb = (r << 16) | (g << 8) | b;
                    indices[y * cw + x] = color_map.get(&rgb).copied().unwrap_or(0);
                } else if a == 0 {
                    indices[y * cw + x] = 0;
                } else {
                    let rgb = (r << 16) | (g << 8) | b;
                    indices[y * cw + x] = color_map.get(&rgb).copied().unwrap_or(0);
                }
            }
        }

        let row_major = if let Some(rm) = meta_row_major {
            rm
        } else if pixel_count > 0 {
            let mut row_runs = 0i64;
            let mut prev = -1i64;
            for idx in indices.iter() {
                let v = *idx as i64;
                if v == prev {
                    row_runs += 1;
                }
                prev = v;
            }
            let mut col_runs = 0i64;
            prev = -1;
            for x in 0..cw {
                for y in 0..ch {
                    let v = indices[y * cw + x] as i64;
                    if v == prev {
                        col_runs += 1;
                    }
                    prev = v;
                }
            }
            col_runs > row_runs
        } else {
            false
        };

        sprites.push(SpriteFrame {
            width: content_w,
            height: content_h,
            left_padding: left_pad,
            top_padding: top_pad,
            has_alpha: has_real_alpha,
            row_major,
            indices,
            alphas: if has_real_alpha { Some(alphas) } else { None },
        });
    }

    Ok(Some(SpriteGroup {
        area_width,
        area_height,
        palette,
        raw_palette,
        sprites,
    }))
}

#[cfg(since_647)]
struct RgbaImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

#[cfg(since_647)]
impl Clone for RgbaImage {
    fn clone(&self) -> Self {
        RgbaImage {
            data: self.data.clone(),
            width: self.width,
            height: self.height,
        }
    }
}

#[cfg(since_647)]
struct PngMeta {
    strip_rows: u32,
    has_alpha: Option<bool>,
    row_major: Option<bool>,
    crop: Option<(u16, u16, u16, u16)>,
    palette_hex: Option<String>,
}

#[cfg(since_647)]
fn load_rgba_png_with_meta(path: &Path) -> Result<(RgbaImage, PngMeta)> {
    let file = fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let decoder = png::Decoder::new(BufReader::new(file));
    let mut reader = decoder.read_info()?;
    let info = reader.info().clone();
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| anyhow::anyhow!("{} is too large to decode", path.display()))?;
    let mut buf = vec![0u8; size];
    let frame = reader.next_frame(&mut buf)?;
    let raw = &buf[..frame.buffer_size()];
    let w = info.width;
    let h = info.height;

    let strip_rows = info
        .uncompressed_latin1_text
        .iter()
        .find(|t| t.keyword == "strip_rows")
        .and_then(|t| t.text.parse::<u32>().ok())
        .unwrap_or(0);
    let has_alpha = info
        .uncompressed_latin1_text
        .iter()
        .find(|t| t.keyword == "has_alpha")
        .map(|t| t.text == "1");
    let row_major = info
        .uncompressed_latin1_text
        .iter()
        .find(|t| t.keyword == "row_major")
        .map(|t| t.text == "1");
    let crop = info
        .uncompressed_latin1_text
        .iter()
        .find(|t| t.keyword == "crop")
        .and_then(|t| {
            let parts: Vec<u16> = t.text.split(',').filter_map(|s| s.parse().ok()).collect();
            if parts.len() == 4 {
                Some((parts[0], parts[1], parts[2], parts[3]))
            } else {
                None
            }
        });
    let palette_hex = info
        .uncompressed_latin1_text
        .iter()
        .find(|t| t.keyword == "palette_hex")
        .map(|t| t.text.clone());

    let rgba = decode_to_rgba(raw, &info)?;

    Ok((
        RgbaImage {
            data: rgba,
            width: w,
            height: h,
        },
        PngMeta {
            strip_rows,
            has_alpha,
            row_major,
            crop,
            palette_hex,
        },
    ))
}

#[cfg(since_647)]
fn load_rgba_png(path: &Path) -> Result<RgbaImage> {
    let file = fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let decoder = png::Decoder::new(BufReader::new(file));
    let mut reader = decoder.read_info()?;
    let info = reader.info().clone();
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| anyhow::anyhow!("{} is too large to decode", path.display()))?;
    let mut buf = vec![0u8; size];
    let frame = reader.next_frame(&mut buf)?;
    let raw = &buf[..frame.buffer_size()];
    let w = info.width;
    let h = info.height;

    let rgba = decode_to_rgba(raw, &info)?;
    Ok(RgbaImage {
        data: rgba,
        width: w,
        height: h,
    })
}

#[cfg(since_647)]
fn decode_to_rgba(raw: &[u8], info: &png::Info) -> Result<Vec<u8>> {
    let w = info.width;
    let h = info.height;
    match info.color_type {
        png::ColorType::Rgba => Ok(raw.to_vec()),
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity((w * h * 4) as usize);
            for chunk in raw.chunks(3) {
                out.extend_from_slice(chunk);
                out.push(255);
            }
            Ok(out)
        }
        png::ColorType::Indexed => {
            let plte = info.palette.as_ref().unwrap();
            let trns = info.trns.as_ref();
            let mut out = Vec::with_capacity((w * h * 4) as usize);
            for &idx in raw {
                let i = idx as usize;
                let r = plte.get(i * 3).copied().unwrap_or(0);
                let g = plte.get(i * 3 + 1).copied().unwrap_or(0);
                let b = plte.get(i * 3 + 2).copied().unwrap_or(0);
                let a = trns.and_then(|t| t.get(i)).copied().unwrap_or(255);
                out.extend_from_slice(&[r, g, b, a]);
            }
            Ok(out)
        }
        _ => anyhow::bail!("unsupported color type {:?}", info.color_type),
    }
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
