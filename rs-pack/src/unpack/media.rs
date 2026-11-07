#![allow(clippy::needless_range_loop)]
#[cfg(since_647)]
use crate::archive;
#[cfg(since_647)]
use crate::pack::names::{AssetNames, archive_name};
#[cfg(before_647)]
use crate::{back, unpack};
#[cfg(since_647)]
use crate::{sheet, tga};
#[cfg(since_647)]
use anyhow::{Result, ensure};
#[cfg(before_647)]
use rs_io::jag::JagFile;
#[cfg(since_647)]
use rs_io::js5::Js5Store;
use std::fs;
use std::path::Path;
#[cfg(before_647)]
use tracing::debug;
#[cfg(since_647)]
use tracing::info;

#[cfg(before_647)]
const MEDIA_NAMES: &[&str] = &[
    "backbase1",
    "backbase2",
    "backhmid1",
    "backhmid2",
    "backleft1",
    "backleft2",
    "backright1",
    "backright2",
    "backtop1",
    "backtop2",
    "backvmid1",
    "backvmid2",
    "backvmid3",
    "chatback",
    "combatboxes",
    "combaticons",
    "combaticons2",
    "combaticons3",
    "compass",
    "cross",
    "gnomeball_buttons",
    "headicons",
    "hitmarks",
    "index",
    "invback",
    "leftarrow",
    "magicoff",
    "magicoff2",
    "magicon",
    "magicon2",
    "mapback",
    "mapdots",
    "mapflag",
    "mapfunction",
    "mapscene",
    #[cfg(since_244)]
    "mapedge",
    #[cfg(since_244)]
    "mapmarker",
    #[cfg(since_244)]
    "mod_icons",
    "miscgraphics",
    "miscgraphics2",
    "miscgraphics3",
    "prayerglow",
    "prayeroff",
    "prayeron",
    "redstone1",
    "redstone2",
    "redstone3",
    "rightarrow",
    "scrollbar",
    "sideicons",
    "staticons",
    "staticons2",
    "steelborder",
    "steelborder2",
    "sworddecor",
    "tradebacking",
    "wornicons",
];

#[cfg(before_647)]
pub fn unpack_media(jag: &JagFile, output_dir: &Path) -> anyhow::Result<()> {
    let sprite_dir = output_dir.join("sprites");
    let meta_dir = sprite_dir.join("meta");
    fs::create_dir_all(&sprite_dir)?;
    fs::create_dir_all(&meta_dir)?;

    let index_data = jag
        .read("index.dat")
        .ok_or_else(|| anyhow::anyhow!("Missing index.dat in media JAG"))?;

    let mut sprites: Vec<(String, Vec<u8>)> = Vec::new();

    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        if hash == JagFile::hash("index.dat") {
            continue;
        }
        if let Some(name) = find_media_name(hash)
            && let Some(dat) = jag.read(&format!("{name}.dat"))
        {
            sprites.push((name.to_string(), dat.data));
        }
    }

    let mut index_positions: Vec<(String, u16, Vec<u8>)> = sprites
        .into_iter()
        .map(|(name, data)| {
            let pos = if data.len() >= 2 {
                ((data[0] as u16) << 8) | data[1] as u16
            } else {
                0
            };
            (name, pos, data)
        })
        .collect();

    index_positions.sort_by_key(|(_, pos, _)| *pos);

    let mut keys = Vec::new();
    for (name, _, dat_data) in &index_positions {
        if let Some(mut g) = unpack::decode_group(&index_data.data, dat_data) {
            if !back::FRAME.iter().any(|&(n, ..)| n == name) {
                g.detect_scan_cols(name);
            }
            unpack::write_group_sheet(&sprite_dir.join(format!("{name}.tga")), &g)?;
            keys.push(name.clone());
        }
    }
    let count = keys.len();
    unpack::write_index_order(&sprite_dir, &keys)?;

    let mut sprite_order_entries = Vec::new();
    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        if hash == JagFile::hash("index.dat") {
            sprite_order_entries.push("index".to_string());
        } else if let Some(name) = find_media_name(hash) {
            sprite_order_entries.push(name.to_string());
        }
    }
    let sprite_order_content = sprite_order_entries.join("\n") + "\n";
    fs::write(meta_dir.join("sprite.order"), &sprite_order_content)?;

    debug!("Unpacked {count} sprite groups from media JAG");
    Ok(())
}

#[cfg(before_647)]
fn find_media_name(hash: i32) -> Option<&'static str> {
    for name in MEDIA_NAMES {
        if JagFile::hash(&format!("{name}.dat")) == hash {
            return Some(name);
        }
    }
    None
}

#[cfg(before_647)]
pub(crate) fn known_hashes() -> Vec<i32> {
    let mut hashes = vec![JagFile::hash("index.dat")];
    hashes.extend(
        MEDIA_NAMES
            .iter()
            .map(|n| JagFile::hash(&format!("{n}.dat"))),
    );
    hashes
}

#[cfg(since_647)]
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

#[cfg(since_647)]
pub fn write_sprite_group(data: &[u8], path: &Path) -> Result<usize> {
    let output = |extension: &str| {
        let mut name = path.as_os_str().to_os_string();
        name.push(format!(".{extension}"));
        std::path::PathBuf::from(name)
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if data.starts_with(&[0xff, 0xd8]) {
        fs::write(output("jpg"), data)?;
        return Ok(1);
    }
    let (sprites, width, height, _) = decode_sprites(data)?;
    ensure!(!sprites.is_empty(), "Empty sprite group");
    let (cols, rows) = sheet::grid(width as usize, height as usize, sprites.len());
    let (w, h) = (cols * width as usize, rows * height as usize);
    ensure!(
        w <= u16::MAX as usize && h <= u16::MAX as usize,
        "Sprite sheet too large"
    );
    let mut pixels = vec![0; w * h];
    let mut alpha = vec![0; w * h];
    let mut text = format!(
        "width={width}
height={height}
"
    );
    for (i, sprite) in sprites.iter().enumerate() {
        text.push_str(&format!(
            "frame={},{},{},{},{},{}
",
            sprite.left_padding,
            sprite.top_padding,
            sprite.width,
            sprite.height,
            if sprite.row_major { "columns" } else { "rows" },
            if sprite.has_alpha { "alpha" } else { "opaque" }
        ));
        ensure!(
            sprite.left_padding + sprite.width <= width
                && sprite.top_padding + sprite.height <= height,
            "Sprite crop outside canvas"
        );
        let ox = i % cols * width as usize + sprite.left_padding as usize;
        let oy = i / cols * height as usize + sprite.top_padding as usize;
        for y in 0..sprite.height as usize {
            let src = y * sprite.width as usize;
            let dst = (oy + y) * w + ox;
            pixels[dst..dst + sprite.width as usize]
                .copy_from_slice(&sprite.indices[src..src + sprite.width as usize]);
            if let Some(values) = &sprite.alphas {
                alpha[dst..dst + sprite.width as usize]
                    .copy_from_slice(&values[src..src + sprite.width as usize]);
            }
        }
    }
    let mut palette = vec![255, 0, 255];
    for rgb in sprites[0].palette.iter().skip(1) {
        palette.extend_from_slice(&[(*rgb >> 16) as u8, (*rgb >> 8) as u8, *rgb as u8]);
    }
    tga::write(&output("tga"), &[], &palette, w as u32, h as u32, &pixels)?;
    if sprites.iter().any(|s| s.has_alpha) {
        let palette: Vec<u8> = (0..=255u8).flat_map(|v| [v, v, v]).collect();
        tga::write(
            &output("alpha.tga"),
            &[],
            &palette,
            w as u32,
            h as u32,
            &alpha,
        )?;
    }
    fs::write(output("sprite"), text)?;
    Ok(sprites.len())
}

#[cfg(since_647)]
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

#[cfg(since_647)]
pub fn unpack_sprites(
    store: &mut Js5Store,
    archive_id: u8,
    dir_name: &str,
    output_dir: &Path,
    pack_dir: &Path,
) -> Result<UnpackSpriteStats> {
    let archive_dir = output_dir.join(dir_name);
    let names = AssetNames::load(pack_dir)?;
    fs::create_dir_all(&archive_dir)?;

    let group_count = store.group_count(archive_id).unwrap_or(0);

    let mut stats = UnpackSpriteStats::default();

    for gid in 0..group_count {
        let Some(group) = store.read_group(archive_id, gid)? else {
            continue;
        };
        anyhow::ensure!(
            group.entries.len() == 1,
            "Sprite group {archive_id}/{gid} has multiple entries"
        );
        let entry = &group.entries[0];
        let name = names.name(archive_name(archive_id), gid);
        stats.sprites += write_sprite_group(entry.data(), &archive_dir.join(name))? as u32;
        stats.groups += 1;
    }

    Ok(stats)
}

#[cfg(since_647)]
pub fn unpack_all_sprites(
    store: &mut Js5Store,
    content_dir: &Path,
    pack_dir: &Path,
) -> Result<UnpackSpriteStats> {
    let mut total = UnpackSpriteStats::default();

    info!("Unpacking sprites (archive {})...", archive::SPRITES);
    let s = unpack_sprites(store, archive::SPRITES, "sprites", content_dir, pack_dir)?;
    info!("  {} groups, {} sprites", s.groups, s.sprites);
    total.groups += s.groups;
    total.sprites += s.sprites;

    for (aid, name) in [
        (archive::LOADING_SPRITES, "loading_sprites"),
        (archive::LOADING_SPRITES_RAW, "loading_sprites_raw"),
    ] {
        if !store.has_archive(aid) {
            continue;
        }
        info!("Unpacking loading sprites (archive {aid})...");
        let s = unpack_sprites(store, aid, name, content_dir, pack_dir)?;
        info!("  {} groups, {} sprites", s.groups, s.sprites);
        total.groups += s.groups;
        total.sprites += s.sprites;
    }

    Ok(total)
}

#[cfg(since_647)]
#[derive(Default)]
pub struct UnpackSpriteStats {
    pub groups: u32,
    pub sprites: u32,
}
