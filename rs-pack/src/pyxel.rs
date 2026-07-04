use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};

use crate::unpack::{DecodedGroup, write_group_sheet};

type Rgb = (u8, u8, u8);
type PaletteIndex = HashMap<Rgb, u8>;

const CATEGORIES: [&str; 4] = ["sprites", "textures", "title", "fonts"];

#[cfg(rev = "225")]
const FRAME: &[(&str, usize, usize, usize, usize)] = &[
    ("backleft1", 0, 11, 8, 334),
    ("backleft2", 0, 375, 22, 96),
    ("backright1", 729, 5, 60, 166),
    ("backright2", 752, 231, 37, 261),
    ("backtop1", 0, 0, 561, 11),
    ("backtop2", 561, 0, 228, 5),
    ("backvmid1", 520, 11, 41, 154),
    ("backvmid2", 520, 231, 42, 114),
    ("backvmid3", 501, 375, 61, 117),
    ("backhmid2", 0, 345, 562, 30),
    ("backhmid1", 520, 165, 269, 66), // sub-buffer Rt -> screen (520,165)
    ("backbase1", 0, 471, 501, 61),   // sub-buffer Pt -> screen (0,471)
    ("backbase2", 501, 492, 288, 40), // sub-buffer Qt -> screen (501,492)
];
#[cfg(rev = "225")]
const CANVAS_W: usize = 789;
#[cfg(rev = "225")]
const CANVAS_H: usize = 532;

#[cfg(since_244)]
const FRAME: &[(&str, usize, usize, usize, usize)] = &[
    ("backtop1", 0, 0, 765, 4),
    ("backleft1", 0, 4, 4, 334),
    ("backright1", 722, 4, 43, 156),
    ("backvmid1", 516, 4, 34, 156),
    ("backhmid1", 516, 160, 249, 45),
    ("backvmid2", 516, 205, 37, 133),
    ("backright2", 743, 205, 22, 261),
    ("backhmid2", 0, 338, 553, 19),
    ("backleft2", 0, 357, 17, 96),
    ("backvmid3", 496, 357, 57, 109),
    ("backbase1", 0, 453, 496, 50),
    ("backbase2", 496, 466, 269, 37),
];
#[cfg(since_244)]
const CANVAS_W: usize = 765;
#[cfg(since_244)]
const CANVAS_H: usize = 503;

fn frame_entry(name: &str) -> Option<(usize, usize, usize, usize)> {
    FRAME
        .iter()
        .find(|(n, ..)| *n == name)
        .map(|&(_, x, y, w, h)| (x, y, w, h))
}

fn frame_order(name: &str) -> usize {
    FRAME
        .iter()
        .position(|(n, ..)| *n == name)
        .unwrap_or(usize::MAX)
}

fn png_encode(w: usize, h: usize, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, w as u32, h as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut writer = enc.write_header().expect("png header");
        writer.write_image_data(rgba).expect("png data");
    }
    out
}

fn png_decode(bytes: &[u8]) -> Result<(usize, usize, Vec<u8>)> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .context("png read_info")?;
    let size = reader
        .output_buffer_size()
        .context("png output size overflow")?;
    let mut buf = vec![0u8; size];
    let info = reader.next_frame(&mut buf)?;
    buf.truncate(info.buffer_size());
    let (w, h) = (info.width as usize, info.height as usize);
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => {
            let mut out = vec![0u8; w * h * 4];
            for i in 0..w * h {
                out[i * 4..i * 4 + 3].copy_from_slice(&buf[i * 3..i * 3 + 3]);
                out[i * 4 + 3] = 255;
            }
            out
        }
        other => bail!("unexpected PNG color type {other:?}"),
    };
    Ok((w, h, rgba))
}

fn zip_write(path: &Path, entries: &[(String, Vec<u8>)]) -> Result<()> {
    let mut zw = zip::ZipWriter::new(std::fs::File::create(path)?);
    for (name, data) in entries {
        zw.start_file(
            name.as_str(),
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )?;
        zw.write_all(data)?;
    }
    zw.finish()?;
    Ok(())
}

fn zip_read(path: &Path) -> Result<HashMap<String, Vec<u8>>> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
    let mut map = HashMap::new();
    for i in 0..archive.len() {
        let mut f = archive.by_index(i)?;
        let name = f.name().to_string();
        let mut data = Vec::new();
        f.read_to_end(&mut data)?;
        map.insert(name, data);
    }
    Ok(map)
}

fn frame_to_rgba(frame: &[u8], palette: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; frame.len() * 4];
    for (p, &idx) in frame.iter().enumerate() {
        if idx == 0 {
            continue; // magenta -> transparent
        }
        let (c, o) = (idx as usize * 3, p * 4);
        out[o..o + 3].copy_from_slice(&palette[c..c + 3]);
        out[o + 3] = 255;
    }
    out
}

fn build_rgb2idx(palette: &[u8]) -> PaletteIndex {
    let mut m = HashMap::new();
    for i in 1..palette.len() / 3 {
        let c = i * 3;
        m.entry((palette[c], palette[c + 1], palette[c + 2]))
            .or_insert(i as u8);
    }
    m
}

fn resolve_palette(
    mut palette: Vec<u8>,
    frame_rgba: &[Vec<u8>],
) -> Result<(Vec<u8>, PaletteIndex)> {
    let mut idx = build_rgb2idx(&palette);
    for rgba in frame_rgba {
        for p in 0..rgba.len() / 4 {
            let o = p * 4;
            let key = (rgba[o], rgba[o + 1], rgba[o + 2]);
            if rgba[o + 3] == 0 || key == (255, 0, 255) {
                continue;
            }
            if let std::collections::hash_map::Entry::Vacant(slot) = idx.entry(key) {
                let next = palette.len() / 3;
                if next > u8::MAX as usize {
                    bail!(
                        "group uses more than 256 colors; indexed sprites cap at 256 palette entries"
                    );
                }
                palette.extend_from_slice(&[key.0, key.1, key.2]);
                slot.insert(next as u8);
            }
        }
    }
    Ok((palette, idx))
}

fn rgba_to_frame(rgba: &[u8], rgb2idx: &PaletteIndex) -> Vec<u8> {
    (0..rgba.len() / 4)
        .map(|p| {
            let o = p * 4;
            if rgba[o + 3] == 0 {
                0
            } else {
                *rgb2idx
                    .get(&(rgba[o], rgba[o + 1], rgba[o + 2]))
                    .unwrap_or(&0)
            }
        })
        .collect()
}

fn palette_json(palette: &[u8]) -> Value {
    let n = palette.len() / 3;
    let mut colors = serde_json::Map::new();
    for i in 0..n {
        let c = i * 3;
        colors.insert(
            i.to_string(),
            Value::String(format!(
                "ff{:02x}{:02x}{:02x}",
                palette[c],
                palette[c + 1],
                palette[c + 2]
            )),
        );
    }
    json!({ "width": 8, "height": n.div_ceil(8).max(1), "numColors": n, "colors": colors })
}

fn layer_json(name: &str, tile_refs: Value) -> Value {
    json!({
        "blendMode": "normal", "soloed": false, "parentIndex": -1, "alpha": 255,
        "hidden": false, "muted": false, "name": name, "collapsed": false,
        "type": "tile_layer", "tileRefs": tile_refs,
    })
}

fn doc_json(
    name: &str,
    tw: usize,
    th: usize,
    cw: usize,
    ch: usize,
    num_tiles: usize,
    tiles_wide: usize,
    layers: Value,
    num_layers: usize,
    palette: &[u8],
) -> Value {
    json!({
        "tileset": { "tileHeight": th, "tileWidth": tw, "numTiles": num_tiles, "fixedWidth": true, "tilesWide": tiles_wide },
        "canvas": { "layers": layers, "width": cw, "height": ch, "tileHeight": th, "tileWidth": tw, "numLayers": num_layers, "currentLayerIndex": 0 },
        "palette": palette_json(palette),
        "version": "0.4.95",
        "name": name,
        "animations": {},
        "settings": {},
    })
}

fn write_group(path: &Path, name: &str, g: &DecodedGroup) -> Result<()> {
    let (tw, th) = (g.tile_w, g.tile_h);
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    if g.frames.len() > 1 {
        let (cols, rows) = crate::sheet::grid(tw, th, g.frames.len());
        let (cw, ch) = (cols * tw, rows * th);
        let mut tile_refs = serde_json::Map::new();
        let mut composite = vec![0u8; cw * ch * 4];
        let mut tiles: Vec<(String, Vec<u8>)> = Vec::new();
        for (i, frame) in g.frames.iter().enumerate() {
            let rgba = frame_to_rgba(frame, &g.palette);
            let (col, row) = (i % cols, i / cols);
            for y in 0..th {
                let d = ((row * th + y) * cw + col * tw) * 4;
                composite[d..d + tw * 4].copy_from_slice(&rgba[y * tw * 4..(y + 1) * tw * 4]);
            }
            tile_refs.insert(
                i.to_string(),
                json!({ "index": i, "rot": 0, "flipX": false }),
            );
            tiles.push((format!("tile{i}.png"), png_encode(tw, th, &rgba)));
        }
        let layers = json!({ "0": layer_json(name, Value::Object(tile_refs)) });
        let doc = doc_json(
            name,
            tw,
            th,
            cw,
            ch,
            g.frames.len(),
            cols,
            layers,
            1,
            &g.palette,
        );
        entries.push(("docData.json".into(), serde_json::to_vec(&doc)?));
        entries.push(("layer0.png".into(), png_encode(cw, ch, &composite)));
        entries.extend(tiles);
    } else {
        let layers = json!({ "0": layer_json(name, json!({})) });
        let doc = doc_json(name, tw, th, tw, th, 1, 8, layers, 1, &g.palette);
        entries.push(("docData.json".into(), serde_json::to_vec(&doc)?));
        entries.push((
            "layer0.png".into(),
            png_encode(tw, th, &frame_to_rgba(&g.frames[0], &g.palette)),
        ));
        entries.push((
            "tile0.png".into(),
            png_encode(tw, th, &vec![0u8; tw * th * 4]),
        )); // spare empty tile
    }
    zip_write(path, &entries)
}

fn write_back(path: &Path, pieces: &[(String, DecodedGroup)]) -> Result<()> {
    let palette = &pieces[0].1.palette;
    let mut layer_pngs: Vec<(String, Vec<u8>)> = Vec::new();
    let mut layers = serde_json::Map::new();
    for (i, (name, g)) in pieces.iter().enumerate() {
        let (ox, oy, _, _) =
            frame_entry(name).with_context(|| format!("unknown back piece {name}"))?;
        let piece = frame_to_rgba(&g.frames[0], palette);
        let mut canvas = vec![0u8; CANVAS_W * CANVAS_H * 4];
        for y in 0..g.tile_h {
            for x in 0..g.tile_w {
                let s = (y * g.tile_w + x) * 4;
                let d = ((oy + y) * CANVAS_W + (ox + x)) * 4;
                canvas[d..d + 4].copy_from_slice(&piece[s..s + 4]);
            }
        }
        layer_pngs.push((
            format!("layer{i}.png"),
            png_encode(CANVAS_W, CANVAS_H, &canvas),
        ));
        layers.insert(i.to_string(), layer_json(name, json!({})));
    }
    let doc = doc_json(
        "back",
        CANVAS_W,
        CANVAS_H,
        CANVAS_W,
        CANVAS_H,
        1,
        8,
        Value::Object(layers),
        pieces.len(),
        palette,
    );
    let mut entries = vec![("docData.json".to_string(), serde_json::to_vec(&doc)?)];
    entries.extend(layer_pngs);
    entries.push((
        "tile0.png".into(),
        png_encode(CANVAS_W, CANVAS_H, &vec![0u8; CANVAS_W * CANVAS_H * 4]),
    ));
    zip_write(path, &entries)
}

fn transform_tile(rgba: &[u8], w: usize, h: usize, r: &Value) -> Vec<u8> {
    let flip_x = r.get("flipX").and_then(Value::as_bool).unwrap_or(false);
    let rot = r.get("rot").and_then(Value::as_u64).unwrap_or(0) % 4;
    if !flip_x && rot == 0 {
        return rgba.to_vec();
    }
    let mut cur = rgba.to_vec();
    let (mut cw, mut ch) = (w, h);
    if flip_x {
        let mut out = vec![0u8; cur.len()];
        for y in 0..ch {
            for x in 0..cw {
                let (s, d) = ((y * cw + x) * 4, (y * cw + (cw - 1 - x)) * 4);
                out[d..d + 4].copy_from_slice(&cur[s..s + 4]);
            }
        }
        cur = out;
    }
    for _ in 0..rot {
        let (nw, nh) = (ch, cw);
        let mut out = vec![0u8; cur.len()];
        for y in 0..ch {
            for x in 0..cw {
                let s = (y * cw + x) * 4;
                let d = (x * nw + (ch - 1 - y)) * 4; // 90 clockwise
                out[d..d + 4].copy_from_slice(&cur[s..s + 4]);
            }
        }
        cur = out;
        cw = nw;
        ch = nh;
    }
    cur
}

fn read_group(path: &Path) -> Result<DecodedGroup> {
    let files = zip_read(path)?;
    let doc: Value = serde_json::from_slice(files.get("docData.json").context("no docData.json")?)?;
    let canvas = &doc["canvas"];
    let tw = canvas["tileWidth"].as_u64().context("tileWidth")? as usize;
    let th = canvas["tileHeight"].as_u64().context("tileHeight")? as usize;

    let refs = canvas["layers"]["0"]
        .get("tileRefs")
        .and_then(Value::as_object);
    let frame_rgba: Vec<Vec<u8>> = match refs {
        Some(refs) if !refs.is_empty() => {
            let num_tiles = doc["tileset"]["numTiles"].as_u64().unwrap_or(0) as usize;
            let mut tiles = Vec::with_capacity(num_tiles);
            for i in 0..num_tiles {
                let png = files
                    .get(&format!("tile{i}.png"))
                    .with_context(|| format!("no tile{i}.png"))?;
                tiles.push(png_decode(png)?.2);
            }
            let mut cells: Vec<usize> = refs.keys().filter_map(|k| k.parse().ok()).collect();
            cells.sort_unstable(); // cell order == frame order
            cells
                .iter()
                .map(|&cell| {
                    let r = &refs[&cell.to_string()];
                    let idx = r["index"].as_u64().unwrap_or(0) as usize;
                    transform_tile(&tiles[idx], tw, th, r)
                })
                .collect()
        }
        _ => vec![png_decode(files.get("layer0.png").context("no layer0.png")?)?.2],
    };

    let (palette, rgb2idx) = resolve_palette(parse_palette(&doc["palette"]), &frame_rgba)?;
    let frames = frame_rgba
        .iter()
        .map(|r| rgba_to_frame(r, &rgb2idx))
        .collect();
    Ok(DecodedGroup {
        tile_w: tw,
        tile_h: th,
        palette,
        frames,
    })
}

fn read_back(path: &Path) -> Result<Vec<(String, DecodedGroup)>> {
    let files = zip_read(path)?;
    let doc: Value = serde_json::from_slice(files.get("docData.json").context("no docData.json")?)?;
    let canvas = &doc["canvas"];
    let num_layers = canvas["numLayers"].as_u64().unwrap_or(0) as usize;

    // Crop each piece out of its full-canvas layer into its own RGBA buffer.
    let mut pieces: Vec<(String, usize, usize, Vec<u8>)> = Vec::with_capacity(num_layers);
    for i in 0..num_layers {
        let name = canvas["layers"][i.to_string()]["name"]
            .as_str()
            .context("layer name")?
            .to_string();
        let (ox, oy, w, h) =
            frame_entry(&name).with_context(|| format!("unknown back piece {name}"))?;
        let (lw, _, rgba) = png_decode(
            files
                .get(&format!("layer{i}.png"))
                .with_context(|| format!("no layer{i}.png"))?,
        )?;
        let mut crop = vec![0u8; w * h * 4];
        for y in 0..h {
            let s = ((oy + y) * lw + ox) * 4;
            crop[y * w * 4..(y + 1) * w * 4].copy_from_slice(&rgba[s..s + w * 4]);
        }
        pieces.push((name, w, h, crop));
    }

    // All pieces share one palette, so resolve it across every crop at once.
    let crops: Vec<Vec<u8>> = pieces.iter().map(|(_, _, _, c)| c.clone()).collect();
    let (palette, rgb2idx) = resolve_palette(parse_palette(&doc["palette"]), &crops)?;
    Ok(pieces
        .into_iter()
        .map(|(name, w, h, crop)| {
            let frame = rgba_to_frame(&crop, &rgb2idx);
            (
                name,
                DecodedGroup {
                    tile_w: w,
                    tile_h: h,
                    palette: palette.clone(),
                    frames: vec![frame],
                },
            )
        })
        .collect())
}

fn parse_palette(pal: &Value) -> Vec<u8> {
    let n = pal["numColors"].as_u64().unwrap_or(0) as usize;
    let colors = &pal["colors"];
    let mut out = vec![0u8; n * 3];
    for i in 0..n {
        let hex = colors[i.to_string()].as_str().unwrap_or("ff000000"); // "aarrggbb"
        for (k, range) in [(0usize, 2..4), (1, 4..6), (2, 6..8)] {
            out[i * 3 + k] = u8::from_str_radix(&hex[range], 16).unwrap_or(0);
        }
    }
    out
}

pub fn content_to_pyxel(content_dir: &Path) -> Result<()> {
    let mut groups = 0usize;
    for cat in CATEGORIES {
        let dir = content_dir.join(cat);
        if !dir.is_dir() {
            continue;
        }
        let out = dir.join("pyxel");
        std::fs::create_dir_all(&out)?;
        let mut tgas: Vec<String> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".tga").map(str::to_string))
            .collect();
        tgas.sort();

        let mut back: Vec<(String, DecodedGroup)> = Vec::new();
        for name in &tgas {
            let parsed = crate::sheet::parse(&crate::tga::read(&dir.join(format!("{name}.tga"))));
            let g = DecodedGroup {
                tile_w: parsed.tile_w,
                tile_h: parsed.tile_h,
                palette: parsed.palette,
                frames: parsed.frames,
            };
            if frame_entry(name).is_some() {
                back.push((name.clone(), g));
            } else {
                write_group(&out.join(format!("{name}.pyxel")), name, &g)?;
                groups += 1;
            }
        }
        if !back.is_empty() {
            back.sort_by_key(|(n, _)| frame_order(n));
            write_back(&out.join("back.pyxel"), &back)?;
            groups += 1;
        }
    }
    tracing::info!("wrote {groups} .pyxel docs under {}", content_dir.display());
    Ok(())
}

pub fn content_from_pyxel(content_dir: &Path) -> Result<()> {
    let mut tgas = 0usize;
    for cat in CATEGORIES {
        let dir = content_dir.join(cat);
        let src = dir.join("pyxel");
        if !src.is_dir() {
            continue;
        }
        let mut names: Vec<String> = std::fs::read_dir(&src)?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter_map(|n| n.strip_suffix(".pyxel").map(str::to_string))
            .collect();
        names.sort();
        for name in &names {
            let path = src.join(format!("{name}.pyxel"));
            if name == "back" {
                for (piece, g) in read_back(&path)? {
                    write_group_sheet(&dir.join(format!("{piece}.tga")), &g)?;
                    tgas += 1;
                }
            } else {
                write_group_sheet(&dir.join(format!("{name}.tga")), &read_group(&path)?)?;
                tgas += 1;
            }
        }
    }
    tracing::info!(
        "rebuilt {tgas} group .tga files under {}",
        content_dir.display()
    );
    Ok(())
}
