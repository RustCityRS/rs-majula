#![allow(clippy::needless_range_loop)]
#[cfg(since_647)]
use crate::archive::TEXTURES as TEXTURE_ARCHIVE;
#[cfg(since_647)]
use crate::cache::texture::{OP_NAMES, TextureDef, parse_texture};
#[cfg(before_647)]
use crate::unpack;
#[cfg(since_647)]
use anyhow::{Context, Result};
#[cfg(before_647)]
use rs_io::jag::JagFile;
#[cfg(since_647)]
use rs_io::js5::Js5Store;
use std::fs;
use std::path::Path;
#[cfg(before_647)]
use tracing::debug;

#[cfg(before_647)]
pub const TEXTURE_NAMES: &[&str] = &[
    "door",
    "water",
    "wall",
    "planks",
    "elfdoor",
    "darkwood",
    "roof",
    "damage",
    "leafytree",
    "treestump",
    "leafybase",
    "mossy",
    "railings",
    "painting1",
    "painting2",
    "marble",
    "wood2",
    "fountain",
    "thatched",
    "cargonet",
    "books",
    "elfroof2",
    "elfwood",
    "mossybricks",
    "water_animated",
    "gungywater",
    "web",
    "elfroof",
    "mossydamage",
    "bamboo",
    "willowtex3",
    "lava",
    "bark",
    "mapletree",
    "yewtree",
    #[cfg(rev = "225")]
    "elfbrick",
    #[cfg(since_244)]
    "empty",
    "elfwall",
    "chainmail",
    "mummy",
    "elfpainting",
    "jungleleaf4",
    "plant",
    "jungleleaf2",
    "plant2",
    "roof2",
    "door2",
    "pebblefloor",
    "rockwall",
    "glyphs",
    "canvas",
];

#[cfg(before_647)]
pub fn unpack_textures(jag: &JagFile, output_dir: &Path, pack_dir: &Path) -> anyhow::Result<()> {
    let tex_dir = output_dir.join("textures");
    let meta_dir = tex_dir.join("meta");
    fs::create_dir_all(&meta_dir)?;

    let index_data = jag
        .read("index.dat")
        .ok_or_else(|| anyhow::anyhow!("Missing index.dat in textures JAG"))?;

    let mut textures: Vec<(String, Vec<u8>)> = Vec::new();

    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        if hash == JagFile::hash("index.dat") {
            continue;
        }
        if let Some(id_str) = find_texture_id(hash) {
            let dat_name = format!("{id_str}.dat");
            if let Some(dat) = jag.read(&dat_name) {
                textures.push((id_str, dat.data));
            }
        }
    }

    let mut index_positions: Vec<(String, u16, Vec<u8>)> = textures
        .into_iter()
        .map(|(id_str, data)| {
            let pos = if data.len() >= 2 {
                ((data[0] as u16) << 8) | data[1] as u16
            } else {
                0
            };
            (id_str, pos, data)
        })
        .collect();

    index_positions.sort_by_key(|(_, pos, _)| *pos);

    let index_order: Vec<String> = index_positions.iter().map(|(n, _, _)| n.clone()).collect();

    let mut keys = Vec::new();
    for (id_str, _, dat_data) in &index_positions {
        if let Some(mut g) = unpack::decode_group(&index_data.data, dat_data) {
            let id: u16 = id_str.parse().unwrap_or(u16::MAX);
            let name = texture_name(id);
            g.detect_scan_cols(&name);
            unpack::write_group_sheet(&tex_dir.join(format!("{name}.tga")), &g)?;
            keys.push(id_str.clone());
        }
    }
    unpack::write_index_order(&tex_dir, &keys)?;

    let mut jag_entry_order = Vec::new();
    for i in 0..jag.file_count {
        let hash = jag.file_hash(i);
        if hash == JagFile::hash("index.dat") {
            jag_entry_order.push("index".to_string());
        } else if let Some(id_str) = find_texture_id(hash) {
            jag_entry_order.push(id_str);
        }
    }
    let texture_order_content = jag_entry_order.join("\n") + "\n";
    fs::write(meta_dir.join("texture.order"), &texture_order_content)?;

    let max_id: u16 = index_order
        .iter()
        .filter_map(|s| s.parse::<u16>().ok())
        .max()
        .unwrap_or(0);
    let mut pack_lines = Vec::new();
    for id in 0..=max_id {
        pack_lines.push(format!("{id}={}", texture_name(id)));
    }
    fs::write(pack_dir.join("texture.pack"), pack_lines.join("\n") + "\n")?;

    debug!("Unpacked {} textures from textures JAG", index_order.len());
    Ok(())
}

#[cfg(before_647)]
fn texture_name(id: u16) -> String {
    TEXTURE_NAMES
        .get(id as usize)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("texture_{id}"))
}

#[cfg(before_647)]
fn find_texture_id(hash: i32) -> Option<String> {
    for id in 0..256u16 {
        if JagFile::hash(&format!("{id}.dat")) == hash {
            return Some(id.to_string());
        }
    }
    None
}

#[cfg(before_647)]
pub(crate) fn known_hashes() -> Vec<i32> {
    let mut hashes = vec![JagFile::hash("index.dat")];
    hashes.extend((0..256).map(|id| JagFile::hash(&format!("{id}.dat"))));
    hashes
}

#[cfg(since_647)]
pub fn unpack_texture_text(
    store: &mut Js5Store,
    content_dir: &Path,
    pack_dir: &Path,
) -> Result<u32> {
    let dir = content_dir.join("textures");
    fs::create_dir_all(&dir)?;
    let names = crate::pack::names::AssetNames::load(pack_dir)?;
    let groups = store
        .reference_table(TEXTURE_ARCHIVE)
        .context("Missing texture table")?
        .groups
        .clone();
    for info in &groups {
        let data = store
            .read_group_raw_decompressed(TEXTURE_ARCHIVE, info.id)?
            .context("Missing texture group")?;
        let text = crate::pack::texture::source::convert(
            &emit_texture_text(&parse_texture(&data)),
            &names,
            true,
        )?;
        let numeric = crate::pack::texture::source::convert(&text, &names, false)?;
        let packed = crate::pack::texture::encode_texture(
            &crate::pack::texture::parse_texture_text(&numeric),
        );
        anyhow::ensure!(
            packed == data,
            "Texture {} text round trip changed bytes",
            info.id
        );
        fs::write(
            dir.join(format!("{}.texture", names.name("texture", info.id))),
            text,
        )?;
    }
    Ok(groups.len() as u32)
}

#[cfg(since_647)]
fn r_u8(d: &[u8]) -> u8 {
    d.first().copied().unwrap_or(0)
}

#[cfg(since_647)]
fn r_u16(d: &[u8]) -> u16 {
    u16::from_be_bytes([
        d.first().copied().unwrap_or(0),
        d.get(1).copied().unwrap_or(0),
    ])
}

#[cfg(since_647)]
fn r_i16(d: &[u8]) -> i16 {
    r_u16(d) as i16
}

#[cfg(since_647)]
fn r_u24(d: &[u8]) -> u32 {
    ((d.first().copied().unwrap_or(0) as u32) << 16)
        | ((d.get(1).copied().unwrap_or(0) as u32) << 8)
        | (d.get(2).copied().unwrap_or(0) as u32)
}

#[cfg(since_647)]
fn emit_field(op: u8, oc: u8, d: &[u8]) -> String {
    match (op, oc) {
        (0, 0) => format!("value={}", r_u8(d)),
        (1, 0) => format!("color=0x{:06x}", r_u24(d)),
        (4, 0) => format!("cols={}", r_u8(d)),
        (4, 1) => format!("rows={}", r_u8(d)),
        (4, 2) => format!("col_jitter={}", r_u16(d)),
        (4, 3) => format!("row_jitter={}", r_u16(d)),
        (4, 4) => format!("shear={}", r_u16(d)),
        (4, 5) => format!("v_offset={}", r_u16(d)),
        (4, 6) => format!("gap={}", r_u16(d)),
        (4, 7) => format!("brightness_range={}", r_u16(d)),
        (5, 0) => format!("radius_x={}", r_u8(d)),
        (5, 1) => format!("radius_y={}", r_u8(d)),
        (5, 2) => format!("monochrome={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (6, 0) => format!("lower={}", r_u16(d)),
        (6, 1) => format!("upper={}", r_u16(d)),
        (6, 2) => format!("monochrome={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (7, 0) => format!("op={}", r_u8(d)),
        (7, 1) => format!("monochrome={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (8, 0) => format!("markers={}", hex_encode(d)),
        (9, 0) => format!("mirror_h={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (9, 1) => format!("mirror_v={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (9, 2) => format!("monochrome={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (10, 0) => format!("stops={}", hex_encode(d)),
        (11, 0) => format!("red={}", r_u16(d)),
        (11, 1) => format!("green={}", r_u16(d)),
        (11, 2) => format!("blue={}", r_u16(d)),
        (12, 0) => format!("mix={}", r_u8(d)),
        (12, 1) => format!("interp={}", r_u8(d)),
        (12, 3) => format!("steepness={}", r_u8(d)),
        (14, 0) => format!("border={}", r_u16(d)),
        (15, 0) => format!("tile_count={}", r_u8(d)),
        (15, 1) => format!("seed={}", r_u8(d)),
        (15, 2) => format!("jitter={}", r_u16(d)),
        (15, 3) => format!("distance_mode={}", r_u8(d)),
        (15, 4) => format!("metric={}", r_u8(d)),
        (15, 5) => format!("tile_count_h={}", r_u8(d)),
        (15, 6) => format!("tile_count_v={}", r_u8(d)),
        (16, 0) => format!("count_h={}", r_u8(d)),
        (16, 1) => format!("count_v={}", r_u8(d)),
        (16, 2) => format!("gap={}", r_u16(d)),
        (17, 0) => format!("hue={}", r_i16(d)),
        (17, 1) => format!("saturation={}", r_u8(d) as i8),
        (17, 2) => format!("lightness={}", r_u8(d) as i8),
        (18, 0) | (39, 0) => format!("sprite={}", r_u16(d)),
        (19, 0) => format!("strength={}", r_u16(d)),
        (19, 1) => format!("monochrome={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (20, 0) => format!("count_h={}", r_u8(d)),
        (20, 1) => format!("count_v={}", r_u8(d)),
        (21, 0) | (22, 0) | (23, 0) => {
            format!("monochrome={}", if r_u8(d) == 1 { "yes" } else { "no" })
        }
        (25, 0) => format!("tolerance={}", r_u16(d)),
        (25, 1) => format!("scale_b={}", r_u16(d)),
        (25, 2) => format!("scale_g={}", r_u16(d)),
        (25, 3) => format!("scale_r={}", r_u16(d)),
        (25, 4) => format!("target_color=0x{:06x}", r_u24(d)),
        (26, 0) => format!("lower={}", r_u16(d)),
        (26, 1) => format!("upper={}", r_u16(d)),
        (27, 0) => format!("count={}", r_u8(d)),
        (27, 1) => format!("duty_cycle={}", r_u16(d)),
        (27, 2) => format!("direction={}", r_u8(d)),
        (28, 0) => format!("seed={}", r_u8(d)),
        (28, 1) => format!("min_width={}", r_u16(d)),
        (28, 2) => format!("max_width={}", r_u16(d)),
        (28, 3) => format!("min_height={}", r_u16(d)),
        (28, 4) => format!("max_height={}", r_u16(d)),
        (28, 5) => format!("bevel_width={}", r_u16(d)),
        (28, 6) => format!("bevel_type={}", r_u8(d)),
        (28, 7) => format!("brightness_range={}", r_u16(d)),
        (28, 8) => format!("color_variation={}", r_u16(d)),
        (29, 0) => format!("shapes={}", hex_encode(d)),
        (29, 1) => format!("monochrome={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (30, 0) => format!("min={}", r_u16(d)),
        (30, 1) => format!("max={}", r_u16(d)),
        (30, 2) => format!("monochrome={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (31, 0) => format!("scale={}", r_u16(d)),
        (31, 1) => format!("iterations={}", r_u16(d)),
        (31, 2) => format!("offset_x={}", r_u16(d)),
        (31, 3) => format!("offset_y={}", r_u16(d)),
        (32, 0) => format!("height_scale={}", r_u16(d)),
        (32, 1) => format!("light_yaw={}", r_u16(d)),
        (32, 2) => format!("light_pitch={}", r_u16(d)),
        (33, 0) => format!("height_scale={}", r_u16(d)),
        (33, 1) => format!("bias_unsigned={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (34, 0) => format!("normalize={}", if r_u8(d) == 1 { "yes" } else { "no" }),
        (34, 1) => format!("octaves={}", r_u8(d)),
        (34, 2) => format!("persistence={}", hex_encode(d)),
        (34, 3) => format!("frequency={}", r_u8(d)),
        (34, 4) => format!("seed={}", r_u8(d)),
        (34, 5) => format!("frequency_h={}", r_u8(d)),
        (34, 6) => format!("frequency_v={}", r_u8(d)),
        (35, 0) => format!("strength={}", r_u16(d)),
        (36, 0) => format!("texture_id={}", r_u16(d)),
        (37, 0) => format!("offset_x1={}", r_u16(d)),
        (37, 1) => format!("offset_y1={}", r_u16(d)),
        (37, 2) => format!("offset_x2={}", r_u16(d)),
        (37, 3) => format!("offset_y2={}", r_u16(d)),
        (37, 4) => format!("wave_scale={}", r_u16(d)),
        (37, 5) => format!("amplitude={}", r_u16(d)),
        (37, 6) => format!("frequency={}", r_u16(d)),
        (38, 0) => format!("seed={}", r_u8(d)),
        (38, 1) => format!("line_count={}", r_u16(d)),
        (38, 2) => format!("line_length={}", r_u8(d)),
        (38, 3) => format!("angle_base={}", r_u16(d)),
        (38, 4) => format!("angle_range={}", r_u16(d)),
        _ => {
            if d.is_empty() {
                format!("field_{}=", oc)
            } else {
                format!("field_{}={}", oc, hex_encode(d))
            }
        }
    }
}

#[cfg(since_647)]
pub fn emit_texture_text(def: &TextureDef) -> String {
    let mut out = String::new();

    out.push_str("[texture]\n");
    out.push_str(&format!("color=op{}\n", def.color_op));
    out.push_str(&format!("alpha=op{}\n", def.alpha_op));
    out.push_str(&format!("mod=op{}\n", def.mod_op));
    if !def.trailing_material.is_empty() {
        out.push_str(&format!(
            "trailing={}\n",
            hex_encode(&def.trailing_material)
        ));
    }

    for (i, op) in def.ops.iter().enumerate() {
        out.push('\n');
        out.push_str(&format!("[op{}]\n", i));
        out.push_str(&format!(
            "type={}\n",
            OP_NAMES
                .get(op.op_type as usize)
                .copied()
                .unwrap_or("unknown")
        ));
        out.push_str(&format!("row_cache={}\n", op.cache_size));

        for &(opcode, ref data) in &op.fields {
            out.push_str(&emit_field(op.op_type, opcode, data));
            out.push('\n');
        }

        if let Some(conns) = def.connections.get(i) {
            for (ci, &c) in conns.iter().enumerate() {
                out.push_str(&format!("input{}=op{}\n", ci, c));
            }
        }
    }

    out
}

#[cfg(since_647)]
fn hex_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for &b in data {
        s.push_str(&format!("{:02x}", b));
    }
    s
}
