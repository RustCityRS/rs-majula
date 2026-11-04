//! Render all textures to PNG using the procedural texture evaluator.
//!
//! Parses each texture definition from archive 9, evaluates the full
//! op DAG via `texture_render::render_texture`, and writes RGBA PNGs.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use rs_dat2::CacheStore;
use tracing::{info, warn};

use crate::texture_format::{self, TEXTURE_ARCHIVE};
use crate::texture_render::{self, SpritePixels};

const RENDER_SIZE: usize = 128;
const BRIGHTNESS: f64 = 0.8;

pub struct RenderStats {
    pub total: u32,
    pub rendered: u32,
    pub failed: u32,
}

pub fn render_all_textures(
    store: &mut CacheStore,
    content_dir: &Path,
) -> Result<RenderStats> {
    let tex_dir = content_dir.join("textures").join("_unpack").join("comparison");
    std::fs::create_dir_all(&tex_dir)?;

    let sprites = super::sprites::load_sprite_pixels_from_cache(store)?;
    info!("  loaded {} sprite images for texture rendering", sprites.len());

    let group_count = store.group_count(TEXTURE_ARCHIVE).unwrap_or(0);
    let mut stats = RenderStats { total: 0, rendered: 0, failed: 0 };
    let mut rendered_textures: HashMap<u16, Vec<u32>> = HashMap::new();
    let lowmem_flags = load_lowmem_flags(store)?;

    // First pass: parse all textures, find TexRef dependencies, topologically sort
    let mut tex_defs: HashMap<u32, texture_format::TextureDef> = HashMap::new();
    let mut deps: HashMap<u32, Vec<u16>> = HashMap::new();
    for group_id in 0..group_count {
        let Some(group) = store.read_group(TEXTURE_ARCHIVE, group_id)? else { continue };
        let Some(entry) = group.entries.first() else { continue };
        if entry.data.is_empty() { continue; }
        let tex_def = texture_format::parse_texture(&entry.data);
        let tex_refs: Vec<u16> = tex_def.ops.iter()
            .filter(|op| op.op_type == 36)
            .filter_map(|op| op.texture_ref)
            .collect();
        deps.insert(group_id, tex_refs);
        tex_defs.insert(group_id, tex_def);
    }

    // Topological sort: render textures with no deps first, then those that depend on them
    let mut order: Vec<u32> = Vec::with_capacity(tex_defs.len());
    let mut rendered_set: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut remaining: Vec<u32> = tex_defs.keys().copied().collect();
    remaining.sort();
    let max_passes = remaining.len() + 1;
    for _ in 0..max_passes {
        if remaining.is_empty() { break; }
        let mut next_remaining = Vec::new();
        for &gid in &remaining {
            let all_deps_met = deps.get(&gid).map_or(true, |d|
                d.iter().all(|&dep| rendered_set.contains(&(dep as u32)))
            );
            if all_deps_met {
                order.push(gid);
                rendered_set.insert(gid);
            } else {
                next_remaining.push(gid);
            }
        }
        if next_remaining.len() == remaining.len() {
            order.extend(next_remaining.iter());
            break;
        }
        remaining = next_remaining;
    }

    for &group_id in &order {
        let tex_def = tex_defs.get(&group_id).unwrap();
        stats.total += 1;

        // Render at TexRef resolution (64 for lowmem materials, 128 otherwise)
        let ref_size = if lowmem_flags.get(&group_id).copied().unwrap_or(false) { 64 } else { 128 };
        if ref_size != RENDER_SIZE {
            let (_, ref_raw) = texture_render::render_texture(
                tex_def, ref_size, BRIGHTNESS, &sprites, &rendered_textures,
            );
            rendered_textures.insert(group_id as u16, ref_raw);
        }

        let (pixels, raw) = texture_render::render_texture(
            tex_def, RENDER_SIZE, BRIGHTNESS, &sprites, &rendered_textures,
        );
        if ref_size == RENDER_SIZE {
            rendered_textures.insert(group_id as u16, raw);
        }

        let out_path = tex_dir.join(format!("{}.png", group_id));
        match write_rgba_png(&out_path, &pixels, RENDER_SIZE, RENDER_SIZE) {
            Ok(_) => stats.rendered += 1,
            Err(e) => {
                warn!("texture {}: render failed: {}", group_id, e);
                stats.failed += 1;
            }
        }
    }

    info!(
        "Rendered {} textures ({} OK, {} failed)",
        stats.total, stats.rendered, stats.failed
    );

    // Dump op usage diagnostic
    let diag_path = content_dir.join("textures").join("_unpack").join("texture_ops.tsv");
    let mut diag = String::from("id\tops\tsprites\n");
    for group_id in 0..group_count {
        let Some(group) = store.read_group(TEXTURE_ARCHIVE, group_id)? else { continue };
        let Some(entry) = group.entries.first() else { continue };
        if entry.data.is_empty() { continue; }
        let tex_def = texture_format::parse_texture(&entry.data);
        let ops: Vec<String> = tex_def.ops.iter().map(|o| o.op_type.to_string()).collect();
        let sprite_ids: Vec<String> = tex_def.ops.iter().filter_map(|o| o.sprite_id.map(|s| s.to_string())).collect();
        diag.push_str(&format!("{}\t{}\t{}\n", group_id, ops.join(","), sprite_ids.join(",")));
    }
    std::fs::write(&diag_path, diag)?;
    Ok(stats)
}

fn load_lowmem_flags(store: &mut CacheStore) -> Result<HashMap<u32, bool>> {
    let mut flags = HashMap::new();
    let Some(group) = store.read_group(26, 0)? else { return Ok(flags) };
    let Some(entry) = group.entries.first() else { return Ok(flags) };
    let data = &entry.data;
    if data.len() < 2 { return Ok(flags); }
    let mut pos = 0;
    let count = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;

    let mut has_mat = vec![false; count];
    for i in 0..count { has_mat[i] = data.get(pos).copied().unwrap_or(0) == 1; pos += 1; }

    // Skip highdetail
    for i in 0..count { if has_mat[i] { pos += 1; } }

    // Read is_lowmem
    for i in 0..count {
        if has_mat[i] {
            flags.insert(i as u32, data.get(pos).copied().unwrap_or(0) == 1);
            pos += 1;
        }
    }
    Ok(flags)
}

fn load_sprite_pixels(content_dir: &Path) -> Result<HashMap<u16, SpritePixels>> {
    let sprite_dir = content_dir.join("sprites").join("_unpack");
    let mut sprites = HashMap::new();

    if !sprite_dir.exists() {
        return Ok(sprites);
    }

    for entry in std::fs::read_dir(&sprite_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() { continue; }
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("png") { continue; }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let id: u16 = match stem.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Ok(sp) = load_png_as_sprite(&path) {
            sprites.insert(id, sp);
        }
    }

    // Also check subdirectories (sprite_id/0.png pattern)
    for entry in std::fs::read_dir(&sprite_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() { continue; }
        let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let id: u16 = match dir_name.parse() {
            Ok(v) => v,
            Err(_) => continue,
        };
        if sprites.contains_key(&id) { continue; }
        let sub_png = path.join("0.png");
        if sub_png.exists() {
            if let Ok(sp) = load_png_as_sprite(&sub_png) {
                sprites.insert(id, sp);
            }
        }
    }

    Ok(sprites)
}

fn load_png_as_sprite(path: &Path) -> Result<SpritePixels> {
    let file = std::fs::File::open(path)
        .with_context(|| format!("open {}", path.display()))?;
    let mut decoder = png::Decoder::new(file);
    // Expand indexed/palette PNGs to RGB, and grayscale to RGB
    decoder.set_transformations(png::Transformations::EXPAND);
    let mut reader = decoder.read_info()
        .with_context(|| format!("decode PNG header {}", path.display()))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let frame = reader.next_frame(&mut buf)
        .with_context(|| format!("read PNG frame {}", path.display()))?;

    let width = frame.width as usize;
    let height = frame.height as usize;
    let data = &buf[..frame.buffer_size()];

    let mut pixels = vec![0u32; width * height];
    match frame.color_type {
        png::ColorType::Rgba => {
            for i in 0..width * height {
                let r = data[i * 4] as u32;
                let g = data[i * 4 + 1] as u32;
                let b = data[i * 4 + 2] as u32;
                pixels[i] = (r << 16) | (g << 8) | b;
            }
        }
        png::ColorType::Rgb => {
            for i in 0..width * height {
                let r = data[i * 3] as u32;
                let g = data[i * 3 + 1] as u32;
                let b = data[i * 3 + 2] as u32;
                pixels[i] = (r << 16) | (g << 8) | b;
            }
        }
        png::ColorType::Grayscale => {
            for i in 0..width * height {
                let v = data[i] as u32;
                pixels[i] = (v << 16) | (v << 8) | v;
            }
        }
        png::ColorType::GrayscaleAlpha => {
            for i in 0..width * height {
                let v = data[i * 2] as u32;
                pixels[i] = (v << 16) | (v << 8) | v;
            }
        }
        png::ColorType::Indexed => {
            // EXPAND transform should have converted to RGB already
            for i in 0..width * height {
                let r = data[i * 3] as u32;
                let g = data[i * 3 + 1] as u32;
                let b = data[i * 3 + 2] as u32;
                pixels[i] = (r << 16) | (g << 8) | b;
            }
        }
    }
    Ok(SpritePixels { width, height, pixels })
}

fn write_rgba_png(path: &Path, pixels: &[u8], width: usize, height: usize) -> Result<()> {
    let file = std::fs::File::create(path)
        .with_context(|| format!("create {}", path.display()))?;
    let mut encoder = png::Encoder::new(file, width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(pixels)?;
    Ok(())
}
