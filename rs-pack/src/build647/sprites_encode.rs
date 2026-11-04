#![allow(
    clippy::useless_asref,
    clippy::needless_range_loop,
    clippy::option_as_ref_deref
)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Sprite encoding: read Jordan-style RGBA PNGs with embedded palette strips,
//! derive all metadata (padding, row_major, has_alpha), encode binary format.

use anyhow::Result;

struct SpriteGroup {
    area_width: u16,
    area_height: u16,
    palette: Vec<u32>,
    raw_palette: Vec<u32>,
    sprites: Vec<SpriteFrame>,
}

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

/// Read a group directory of PNGs and derive all sprite metadata.
/// First PNG contains palette strip rows at the bottom (alpha=254 sentinel).
/// Encode a sprite group directory to binary. Public for testing.
/// Handles both indexed sprite groups (PNGs) and raw JPEG images (.jpg).
pub fn encode_sprite_group_from_dir(group_dir: &std::path::Path) -> Result<Option<Vec<u8>>> {
    let jpg_path = group_dir.join("0.jpg");
    if jpg_path.exists() {
        return Ok(Some(std::fs::read(&jpg_path)?));
    }
    match read_sprite_group(group_dir)? {
        Some(group) => Ok(Some(encode_sprite_group_binary(&group))),
        None => Ok(None),
    }
}

fn read_sprite_group(group_dir: &std::path::Path) -> Result<Option<SpriteGroup>> {
    let mut png_files: Vec<(u32, std::path::PathBuf)> = Vec::new();
    for entry in std::fs::read_dir(group_dir)? {
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
                let s = std::str::from_utf8(chunk).unwrap_or("000000");
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
        let mut seen = std::collections::HashSet::new();
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

    let color_map: std::collections::HashMap<u32, u8> = palette
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

struct RgbaImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl Clone for RgbaImage {
    fn clone(&self) -> Self {
        RgbaImage {
            data: self.data.clone(),
            width: self.width,
            height: self.height,
        }
    }
}

struct PngMeta {
    strip_rows: u32,
    has_alpha: Option<bool>,
    row_major: Option<bool>,
    crop: Option<(u16, u16, u16, u16)>,
    palette_hex: Option<String>,
}

fn load_rgba_png_with_meta(path: &std::path::Path) -> Result<(RgbaImage, PngMeta)> {
    use anyhow::Context;
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
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

fn load_rgba_png(path: &std::path::Path) -> Result<RgbaImage> {
    use anyhow::Context;
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
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
            let alpha = sprite
                .alphas
                .as_ref()
                .map(|a| a.as_slice())
                .unwrap_or(&opaque_fallback);
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
    for i in 1..binary_palette.len() {
        let rgb = binary_palette[i];
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
