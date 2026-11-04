#![allow(clippy::needless_range_loop)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Unpack sprite archives (8, 32) into PNG files.
//!
//! Format from `JagexSprite.java` (rev 647 deobfuscated client):
//! Each group contains one or more indexed-color sprites sharing a palette.
//! The binary layout stores pixel data at the front and metadata at the end.
//!
//! Output format (Jordan-style, no .meta files):
//! - Each sprite group → `{gid}/0.png`, `{gid}/1.png`, etc.
//! - PNGs are full area_width × area_height RGBA canvases.
//! - Padding region = alpha 0 (transparent).
//! - Content pixels = actual RGB + alpha 255 (or real alpha for has_alpha sprites).
//! - First PNG has extra palette rows appended at the bottom (alpha=254 sentinel).

use std::io::BufWriter;
use std::path::Path;

use anyhow::{Context, Result};
use rs_dat2::CacheStore;
use tracing::{info, warn};

const ARCHIVE_SPRITES: u8 = 8;
const ARCHIVE_LOADING_SPRITES: u8 = 32;

#[derive(Debug)]
struct Sprite {
    width: u16,
    height: u16,
    left_padding: u16,
    top_padding: u16,
    has_alpha: bool,
    row_major: bool,
    indices: Vec<u8>,
    alphas: Option<Vec<u8>>,
    palette: Vec<u32>,
}

/// Decode a Jagex JPEG (potentially 4-component YCCK) to RGB pixels.
// Used by the texture renderer (textures_render), which is not wired into
// the unpack driver yet — kept for the later texture milestone.
#[allow(dead_code)]
fn decode_jagex_jpeg(data: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    use zune_core::colorspace::ColorSpace;
    use zune_core::options::DecoderOptions;
    use zune_jpeg::JpegDecoder;

    let mut probe = JpegDecoder::new(data);
    probe
        .decode_headers()
        .map_err(|e| anyhow::anyhow!("JPEG headers: {:?}", e))?;
    let input_cs = probe.get_input_colorspace().unwrap_or(ColorSpace::RGB);
    let options = DecoderOptions::default().jpeg_set_out_colorspace(input_cs);
    let mut decoder = JpegDecoder::new_with_options(data, options);
    let pixels = decoder
        .decode()
        .map_err(|e| anyhow::anyhow!("JPEG decode: {:?}", e))?;
    let info = decoder
        .info()
        .ok_or_else(|| anyhow::anyhow!("no JPEG info"))?;
    let w = info.width as u32;
    let h = info.height as u32;

    let actual_components = pixels.len() / (w * h) as usize;
    let pixel_count = (w * h) as usize;
    let mut rgb = vec![0u8; pixel_count * 3];

    if actual_components >= 4 {
        for i in 0..pixel_count {
            let y_val = pixels[i * actual_components] as f32;
            let cb = pixels[i * actual_components + 1] as f32 - 128.0;
            let cr = pixels[i * actual_components + 2] as f32 - 128.0;
            let k = pixels[i * actual_components + 3] as f32;
            let r = (y_val + 1.402 * cr) * k / 255.0;
            let g = (y_val - 0.344136 * cb - 0.714136 * cr) * k / 255.0;
            let b = (y_val + 1.772 * cb) * k / 255.0;
            rgb[i * 3] = r.clamp(0.0, 255.0) as u8;
            rgb[i * 3 + 1] = g.clamp(0.0, 255.0) as u8;
            rgb[i * 3 + 2] = b.clamp(0.0, 255.0) as u8;
        }
    } else if actual_components == 3 {
        rgb.copy_from_slice(&pixels[..pixel_count * 3]);
    } else {
        for i in 0..pixel_count {
            let v = pixels[i.min(pixels.len() - 1)];
            rgb[i * 3] = v;
            rgb[i * 3 + 1] = v;
            rgb[i * 3 + 2] = v;
        }
    }

    Ok((rgb, w, h))
}

#[allow(dead_code)]
fn write_rgb_png(rgb: &[u8], width: u32, height: u32, path: &Path) -> Result<()> {
    let file =
        std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(rgb)?;
    Ok(())
}

/// Write a sprite as an RGBA PNG at full area_width × area_height canvas.
/// If `palette_strip` is Some, appends extra rows at the bottom encoding the palette
/// with alpha=254 sentinel (Jordan-style embedded palette).
fn write_sprite_canvas_png(
    sprite: &Sprite,
    area_width: u16,
    area_height: u16,
    palette_strip: Option<&[u32]>,
    path: &Path,
) -> Result<()> {
    let aw = area_width as usize;
    let ah = area_height as usize;

    let strip_rows = if let Some(pal) = palette_strip {
        let colors = pal.len().saturating_sub(1); // skip index 0
        if aw == 0 { 0 } else { colors.div_ceil(aw) }
    } else {
        0
    };

    let total_height = ah + strip_rows;
    let mut rgba = vec![0u8; aw * total_height * 4]; // alpha=0 everywhere (transparent)

    // Place sprite content at (left_padding, top_padding)
    let lp = sprite.left_padding as usize;
    let tp = sprite.top_padding as usize;
    let sw = sprite.width as usize;
    let sh = sprite.height as usize;

    for y in 0..sh {
        for x in 0..sw {
            let src_idx = y * sw + x;
            let palette_idx = sprite.indices[src_idx] as usize;

            let dst_x = lp + x;
            let dst_y = tp + y;
            if dst_x >= aw || dst_y >= ah {
                continue;
            }
            let dst_idx = (dst_y * aw + dst_x) * 4;

            let raw_rgb = sprite.palette.get(palette_idx).copied().unwrap_or(0);
            // Display substitution: 0x000000 → 0x000001 for non-zero indices
            // (avoids palette collision with index 0; raw values in palette_hex tEXt)
            let rgb = if raw_rgb == 0 && palette_idx > 0 {
                1
            } else {
                raw_rgb
            };
            rgba[dst_idx] = ((rgb >> 16) & 0xFF) as u8;
            rgba[dst_idx + 1] = ((rgb >> 8) & 0xFF) as u8;
            rgba[dst_idx + 2] = (rgb & 0xFF) as u8;

            if let Some(ref alphas) = sprite.alphas {
                rgba[dst_idx + 3] = alphas[src_idx];
            } else if palette_idx == 0 {
                // Non-alpha sprite: index 0 = transparent
                rgba[dst_idx + 3] = 0;
            } else {
                rgba[dst_idx + 3] = 255;
            }
        }
    }

    // Write palette strip rows at the bottom (alpha=254 sentinel)
    if let Some(pal) = palette_strip {
        let mut color_idx = 1; // skip palette[0] (transparent)
        for row in 0..strip_rows {
            for col in 0..aw {
                if color_idx >= pal.len() {
                    break;
                }
                let dst_idx = ((ah + row) * aw + col) * 4;
                let rgb = pal[color_idx];
                rgba[dst_idx] = ((rgb >> 16) & 0xFF) as u8;
                rgba[dst_idx + 1] = ((rgb >> 8) & 0xFF) as u8;
                rgba[dst_idx + 2] = (rgb & 0xFF) as u8;
                rgba[dst_idx + 3] = 254; // sentinel
                color_idx += 1;
            }
        }
    }

    let file =
        std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let w = BufWriter::new(file);
    let mut encoder = png::Encoder::new(w, aw as u32, total_height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // Store metadata as PNG text chunks for lossless roundtrip
    if strip_rows > 0 {
        encoder
            .add_text_chunk("strip_rows".to_string(), strip_rows.to_string())
            .ok();
    }
    if sprite.has_alpha {
        encoder
            .add_text_chunk("has_alpha".to_string(), "1".to_string())
            .ok();
    }
    if sprite.row_major {
        encoder
            .add_text_chunk("row_major".to_string(), "1".to_string())
            .ok();
    }
    encoder
        .add_text_chunk(
            "crop".to_string(),
            format!(
                "{},{},{},{}",
                sprite.left_padding, sprite.top_padding, sprite.width, sprite.height
            ),
        )
        .ok();
    // Store raw palette as hex for lossless binary roundtrip (handles 0x000000 collisions)
    if let Some(pal) = palette_strip {
        let hex: String = pal[1..]
            .iter()
            .map(|&rgb| format!("{:06x}", rgb))
            .collect::<Vec<_>>()
            .join("");
        encoder.add_text_chunk("palette_hex".to_string(), hex).ok();
    }
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&rgba)?;
    Ok(())
}

fn decode_sprites(data: &[u8]) -> Result<(Vec<Sprite>, u16, u16, u16)> {
    if data.len() < 2 {
        anyhow::bail!("sprite data too short: {} bytes", data.len());
    }

    let sprite_count = u16::from_be_bytes([data[data.len() - 2], data[data.len() - 1]]);
    if sprite_count == 0 {
        return Ok((vec![], 0, 0, 0));
    }

    let meta_start = data.len() - (sprite_count as usize) * 8 - 7;
    if meta_start >= data.len() {
        anyhow::bail!("sprite metadata offset out of bounds");
    }

    let mut pos = meta_start;
    let area_width = u16::from_be_bytes([data[pos], data[pos + 1]]);
    pos += 2;
    let area_height = u16::from_be_bytes([data[pos], data[pos + 1]]);
    pos += 2;
    let palette_size = (data[pos] as u16) + 1;
    pos += 1;

    let mut left_pads = vec![0u16; sprite_count as usize];
    let mut top_pads = vec![0u16; sprite_count as usize];
    let mut widths = vec![0u16; sprite_count as usize];
    let mut heights = vec![0u16; sprite_count as usize];

    for i in 0..sprite_count as usize {
        left_pads[i] = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;
    }
    for i in 0..sprite_count as usize {
        top_pads[i] = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;
    }
    for i in 0..sprite_count as usize {
        widths[i] = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;
    }
    for i in 0..sprite_count as usize {
        heights[i] = u16::from_be_bytes([data[pos], data[pos + 1]]);
        pos += 2;
    }

    let palette_start =
        data.len() - (sprite_count as usize) * 8 - ((palette_size as usize) - 1) * 3 - 7;
    let mut palette = vec![0u32; palette_size as usize];
    let mut pp = palette_start;
    for i in 1..palette_size as usize {
        let r = data[pp] as u32;
        let g = data[pp + 1] as u32;
        let b = data[pp + 2] as u32;
        pp += 3;
        palette[i] = (r << 16) | (g << 8) | b;
    }

    let mut pixel_pos = 0usize;
    let mut sprites = Vec::with_capacity(sprite_count as usize);

    for i in 0..sprite_count as usize {
        let w = widths[i] as usize;
        let h = heights[i] as usize;
        let pixel_count = w * h;
        let mut indices = vec![0u8; pixel_count];

        if pixel_pos >= data.len() {
            anyhow::bail!("pixel data overflow at sprite {}", i);
        }
        let flags = data[pixel_pos];
        pixel_pos += 1;

        let has_alpha_flag = (flags & 0x2) != 0;
        let row_major = (flags & 0x1) != 0;

        let alphas = if has_alpha_flag {
            let mut alpha_buf = vec![0u8; pixel_count];

            if !row_major {
                for j in 0..pixel_count {
                    indices[j] = data[pixel_pos];
                    pixel_pos += 1;
                }
                for j in 0..pixel_count {
                    alpha_buf[j] = data[pixel_pos];
                    pixel_pos += 1;
                }
            } else {
                for x in 0..w {
                    for y in 0..h {
                        indices[x + y * w] = data[pixel_pos];
                        pixel_pos += 1;
                    }
                }
                for x in 0..w {
                    for y in 0..h {
                        alpha_buf[x + y * w] = data[pixel_pos];
                        pixel_pos += 1;
                    }
                }
            }

            Some(alpha_buf)
        } else {
            if !row_major {
                for j in 0..pixel_count {
                    indices[j] = data[pixel_pos];
                    pixel_pos += 1;
                }
            } else {
                for x in 0..w {
                    for y in 0..h {
                        indices[x + y * w] = data[pixel_pos];
                        pixel_pos += 1;
                    }
                }
            }
            None
        };

        sprites.push(Sprite {
            width: widths[i],
            height: heights[i],
            left_padding: left_pads[i],
            top_padding: top_pads[i],
            has_alpha: has_alpha_flag,
            row_major,
            indices,
            alphas,
            palette: palette.clone(),
        });
    }

    Ok((sprites, area_width, area_height, palette_size))
}

pub fn unpack_sprites(
    store: &mut CacheStore,
    archive_id: u8,
    archive_name: &str,
    output_dir: &Path,
) -> Result<UnpackSpriteStats> {
    let archive_dir = output_dir.join(archive_name).join("_unpack");
    std::fs::create_dir_all(&archive_dir)?;

    let group_count = store.group_count(archive_id).unwrap_or(0);

    let mut stats = UnpackSpriteStats::default();

    for gid in 0..group_count {
        let group = match store.read_group(archive_id, gid) {
            Ok(Some(g)) => g,
            Ok(None) => continue,
            Err(e) => {
                warn!(
                    "archive {} group {}: read failed ({}), skipping",
                    archive_id, gid, e
                );
                stats.failed += 1;
                continue;
            }
        };
        let Some(entry) = group.entries.first() else {
            continue;
        };
        if entry.data.len() < 2 {
            continue;
        }

        // JPEG images (archive 32 mixes sprites and JPEGs)
        // Store as source .jpg files — the build re-packs them directly.
        if entry.data[0] == 0xFF && entry.data[1] == 0xD8 {
            let group_dir = archive_dir.join(gid.to_string());
            std::fs::create_dir_all(&group_dir)?;
            std::fs::write(group_dir.join("0.jpg"), &entry.data)?;
            stats.groups += 1;
            stats.sprites += 1;
            continue;
        }

        let (sprites, area_width, area_height, _palette_size) = match decode_sprites(&entry.data) {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    "archive {} group {}: sprite decode failed ({}) data_len={}",
                    archive_id,
                    gid,
                    e,
                    entry.data.len(),
                );
                stats.failed += 1;
                continue;
            }
        };

        if sprites.is_empty() {
            continue;
        }

        let group_dir = archive_dir.join(gid.to_string());
        std::fs::create_dir_all(&group_dir)?;

        for (file_id, sprite) in sprites.iter().enumerate() {
            let path = group_dir.join(format!("{}.png", file_id));
            // First sprite gets the palette strip
            let palette_strip = if file_id == 0 {
                Some(sprite.palette.as_slice())
            } else {
                None
            };
            write_sprite_canvas_png(sprite, area_width, area_height, palette_strip, &path)?;
        }

        stats.groups += 1;
        stats.sprites += sprites.len() as u32;
    }

    Ok(stats)
}

pub fn unpack_all_sprites(store: &mut CacheStore, content_dir: &Path) -> Result<UnpackSpriteStats> {
    let mut total = UnpackSpriteStats::default();

    info!("Unpacking sprites (archive {})...", ARCHIVE_SPRITES);
    let s = unpack_sprites(store, ARCHIVE_SPRITES, "sprites", content_dir)?;
    info!(
        "  {} groups, {} sprites, {} failed",
        s.groups, s.sprites, s.failed
    );
    total.groups += s.groups;
    total.sprites += s.sprites;
    total.failed += s.failed;

    if store.has_archive(ARCHIVE_LOADING_SPRITES) {
        info!(
            "Unpacking loading sprites (archive {})...",
            ARCHIVE_LOADING_SPRITES
        );
        let s = unpack_sprites(
            store,
            ARCHIVE_LOADING_SPRITES,
            "loading_sprites",
            content_dir,
        )?;
        info!(
            "  {} groups, {} sprites, {} failed",
            s.groups, s.sprites, s.failed
        );
        total.groups += s.groups;
        total.sprites += s.sprites;
        total.failed += s.failed;
    }

    Ok(total)
}

#[derive(Default)]
pub struct UnpackSpriteStats {
    pub groups: u32,
    pub sprites: u32,
    pub failed: u32,
}
