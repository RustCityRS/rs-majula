#[cfg(before_647)]
use super::pack_registry::PackRegistry;
#[cfg(before_647)]
use crate::archive::JagArchive;
#[cfg(since_647)]
use crate::archive::{self, ArchiveVersions};
#[cfg(since_647)]
use crate::cache::js5::PackedCache;
#[cfg(since_647)]
use crate::cache::texture::{OP_NAMES, TextureDef, TextureOp};
#[cfg(since_647)]
use crate::pack::js5::{PreparedGroup, insert_archive};
#[cfg(before_647)]
use crate::pack::util::{media, palette};
#[cfg(since_647)]
use anyhow::Result;
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
pub fn pack_textures_jag(registry: &PackRegistry, content_dir: &Path) -> Vec<u8> {
    let tex_dir = content_dir.join("textures");
    if !tex_dir.exists() {
        panic!("Could not find textures dir");
    }

    let pack = &registry.texture;

    let jag_entry_order: Vec<String> =
        fs::read_to_string(tex_dir.join("meta").join("texture.order"))
            .expect("Missing texture.order")
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect();

    let index_order = media::read_index_order(&tex_dir);
    let mut index: Vec<u8> = Vec::new();
    let mut dat_map: HashMap<String, Vec<u8>> = HashMap::new();

    for id_str in &index_order {
        let name = pack
            .get_by_id(id_str.parse().unwrap_or(u16::MAX))
            .unwrap_or(id_str.as_str());
        let mut group = media::read_group(&tex_dir.join(format!("{name}.tga")));
        palette::derive_group(name, &mut group);
        let data = media::emit_group(&mut index, &group);
        dat_map.insert(id_str.clone(), data);
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

    debug!("Packed textures into textures Jag");
    jag.build(JagArchive::Textures.compression())
}

#[cfg(since_647)]
pub fn pack_archive(
    store: &mut PackedCache,
    content_dir: &Path,
    pack_dir: &Path,
    versions: &HashMap<u8, ArchiveVersions>,
) -> Result<()> {
    let aid = archive::TEXTURES;
    let format = archive::archive_format(aid).unwrap();
    let names = crate::pack::names::AssetNames::load(pack_dir)?;
    let group_files = names.files(
        &content_dir.join(format.content_dir),
        "texture",
        &["texture"],
    )?;
    let prepared: Vec<PreparedGroup> = group_files
        .par_iter()
        .map(|(gid, path)| {
            let text = source::convert(&fs::read_to_string(path)?, &names, false)?;
            let data = encode_texture(&parse_texture_text(&text));
            let gid = *gid;
            let mut group = PreparedGroup::new(aid, gid, &data, versions)?;
            group.files = vec![FileInfo {
                id: 0,
                name_hash: None,
            }];
            Ok(group)
        })
        .collect::<Result<Vec<_>>>()?;

    insert_archive(
        store,
        aid,
        format,
        versions,
        prepared,
        "from .texture encode",
    )?;

    Ok(())
}

#[cfg(since_647)]
pub fn encode_texture(def: &TextureDef) -> Vec<u8> {
    let mut out = Vec::new();
    out.push(def.ops.len() as u8);

    for (i, op) in def.ops.iter().enumerate() {
        out.push(i as u8);
        out.push(op.op_type);
        out.push(op.cache_size);
        out.push(op.fields.len() as u8);

        for &(opcode, ref field_data) in &op.fields {
            out.push(opcode);
            out.extend_from_slice(field_data);
        }

        if let Some(conns) = def.connections.get(i) {
            for &c in conns {
                out.push(c);
            }
        }
    }

    out.push(def.color_op);
    out.push(def.alpha_op);
    out.push(def.mod_op);
    out.extend_from_slice(&def.trailing_material);

    out
}

#[cfg(since_647)]
fn w_u8(v: u8) -> Vec<u8> {
    vec![v]
}

#[cfg(since_647)]
fn w_u16(v: u16) -> Vec<u8> {
    v.to_be_bytes().to_vec()
}

#[cfg(since_647)]
fn w_i16(v: i16) -> Vec<u8> {
    (v as u16).to_be_bytes().to_vec()
}

#[cfg(since_647)]
fn w_u24(v: u32) -> Vec<u8> {
    vec![(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

#[cfg(since_647)]
fn parse_named_field(op: u8, key: &str, val: &str) -> Option<(u8, Vec<u8>)> {
    let b = |s: &str| -> bool { s == "yes" || s == "1" || s == "true" };
    let u8v = |s: &str| -> Vec<u8> { w_u8(s.parse().unwrap_or(0)) };
    let u16v = |s: &str| -> Vec<u8> { w_u16(s.parse().unwrap_or(0)) };
    let i16v = |s: &str| -> Vec<u8> { w_i16(s.parse().unwrap_or(0)) };
    let bv = |s: &str| -> Vec<u8> { w_u8(if b(s) { 1 } else { 0 }) };
    let hex_color = |s: &str| -> Vec<u8> {
        let s = s
            .strip_prefix("0x")
            .or_else(|| s.strip_prefix("0X"))
            .unwrap_or(s);
        w_u24(u32::from_str_radix(s, 16).unwrap_or(0))
    };

    Some(match (op, key) {
        (0, "value") => (0, u8v(val)),
        (1, "color") => (0, hex_color(val)),
        (4, "cols") => (0, u8v(val)),
        (4, "rows") => (1, u8v(val)),
        (4, "col_jitter") => (2, u16v(val)),
        (4, "row_jitter") => (3, u16v(val)),
        (4, "shear") => (4, u16v(val)),
        (4, "v_offset") => (5, u16v(val)),
        (4, "gap") => (6, u16v(val)),
        (4, "brightness_range") => (7, u16v(val)),
        (5, "radius_x") => (0, u8v(val)),
        (5, "radius_y") => (1, u8v(val)),
        (5, "monochrome") => (2, bv(val)),
        (6, "lower") => (0, u16v(val)),
        (6, "upper") => (1, u16v(val)),
        (6, "monochrome") => (2, bv(val)),
        (7, "op") => (0, u8v(val)),
        (7, "monochrome") => (1, bv(val)),
        (8, "markers") => (0, hex_decode(val)),
        (9, "mirror_h") => (0, bv(val)),
        (9, "mirror_v") => (1, bv(val)),
        (9, "monochrome") => (2, bv(val)),
        (10, "stops") => (0, hex_decode(val)),
        (11, "red") => (0, u16v(val)),
        (11, "green") => (1, u16v(val)),
        (11, "blue") => (2, u16v(val)),
        (12, "mix") => (0, u8v(val)),
        (12, "interp") => (1, u8v(val)),
        (12, "steepness") => (3, u8v(val)),
        (14, "border") => (0, u16v(val)),
        (15, "tile_count") => (0, u8v(val)),
        (15, "seed") => (1, u8v(val)),
        (15, "jitter") => (2, u16v(val)),
        (15, "distance_mode") => (3, u8v(val)),
        (15, "metric") => (4, u8v(val)),
        (15, "tile_count_h") => (5, u8v(val)),
        (15, "tile_count_v") => (6, u8v(val)),
        (16, "count_h") => (0, u8v(val)),
        (16, "count_v") => (1, u8v(val)),
        (16, "gap") => (2, u16v(val)),
        (17, "hue") => (0, i16v(val)),
        (17, "saturation") => (1, vec![val.parse::<i8>().unwrap_or(0) as u8]),
        (17, "lightness") => (2, vec![val.parse::<i8>().unwrap_or(0) as u8]),
        (18, "sprite") | (39, "sprite") => (0, u16v(val)),
        (19, "strength") => (0, u16v(val)),
        (19, "monochrome") => (1, bv(val)),
        (20, "count_h") => (0, u8v(val)),
        (20, "count_v") => (1, u8v(val)),
        (21, "monochrome") | (22, "monochrome") | (23, "monochrome") => (0, bv(val)),
        (25, "tolerance") => (0, u16v(val)),
        (25, "scale_b") => (1, u16v(val)),
        (25, "scale_g") => (2, u16v(val)),
        (25, "scale_r") => (3, u16v(val)),
        (25, "target_color") => (4, hex_color(val)),
        (26, "lower") => (0, u16v(val)),
        (26, "upper") => (1, u16v(val)),
        (27, "count") => (0, u8v(val)),
        (27, "duty_cycle") => (1, u16v(val)),
        (27, "direction") => (2, u8v(val)),
        (28, "seed") => (0, u8v(val)),
        (28, "min_width") => (1, u16v(val)),
        (28, "max_width") => (2, u16v(val)),
        (28, "min_height") => (3, u16v(val)),
        (28, "max_height") => (4, u16v(val)),
        (28, "bevel_width") => (5, u16v(val)),
        (28, "bevel_type") => (6, u8v(val)),
        (28, "brightness_range") => (7, u16v(val)),
        (28, "color_variation") => (8, u16v(val)),
        (29, "shapes") => (0, hex_decode(val)),
        (29, "monochrome") => (1, bv(val)),
        (30, "min") => (0, u16v(val)),
        (30, "max") => (1, u16v(val)),
        (30, "monochrome") => (2, bv(val)),
        (31, "scale") => (0, u16v(val)),
        (31, "iterations") => (1, u16v(val)),
        (31, "offset_x") => (2, u16v(val)),
        (31, "offset_y") => (3, u16v(val)),
        (32, "height_scale") => (0, u16v(val)),
        (32, "light_yaw") => (1, u16v(val)),
        (32, "light_pitch") => (2, u16v(val)),
        (33, "height_scale") => (0, u16v(val)),
        (33, "bias_unsigned") => (1, bv(val)),
        (34, "normalize") => (0, bv(val)),
        (34, "octaves") => (1, u8v(val)),
        (34, "persistence") => (2, hex_decode(val)),
        (34, "frequency") => (3, u8v(val)),
        (34, "seed") => (4, u8v(val)),
        (34, "frequency_h") => (5, u8v(val)),
        (34, "frequency_v") => (6, u8v(val)),
        (35, "strength") => (0, u16v(val)),
        (36, "texture_id") => (0, u16v(val)),
        (37, "offset_x1") => (0, u16v(val)),
        (37, "offset_y1") => (1, u16v(val)),
        (37, "offset_x2") => (2, u16v(val)),
        (37, "offset_y2") => (3, u16v(val)),
        (37, "wave_scale") => (4, u16v(val)),
        (37, "amplitude") => (5, u16v(val)),
        (37, "frequency") => (6, u16v(val)),
        (38, "seed") => (0, u8v(val)),
        (38, "line_count") => (1, u16v(val)),
        (38, "line_length") => (2, u8v(val)),
        (38, "angle_base") => (3, u16v(val)),
        (38, "angle_range") => (4, u16v(val)),
        _ => return None,
    })
}

#[cfg(since_647)]
pub fn parse_texture_text(text: &str) -> TextureDef {
    let mut def = TextureDef {
        ops: Vec::new(),
        connections: Vec::new(),
        color_op: 0,
        alpha_op: 0,
        mod_op: 0,
        trailing_material: Vec::new(),
    };

    enum Section {
        None,
        Texture,
        Op,
    }

    struct OpBuild {
        op_type: u8,
        cache_size: u8,
        fields: Vec<(u8, Vec<u8>)>,
        conns: Vec<u8>,
    }

    let mut section = Section::None;
    let mut op_builds: Vec<(usize, OpBuild)> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed == "[texture]" {
            section = Section::Texture;
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("[op") {
            if let Some(idx_str) = rest.strip_suffix(']') {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    section = Section::Op;
                    op_builds.push((
                        idx,
                        OpBuild {
                            op_type: 0,
                            cache_size: 0,
                            fields: Vec::new(),
                            conns: Vec::new(),
                        },
                    ));
                    continue;
                }
            }
        }

        let Some((key, val)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let val = val.trim();

        match &section {
            Section::Texture => match key {
                "color" => def.color_op = parse_op_ref(val),
                "alpha" => def.alpha_op = parse_op_ref(val),
                "mod" => def.mod_op = parse_op_ref(val),
                "trailing" => def.trailing_material = hex_decode(val),
                _ => {}
            },
            Section::Op => {
                let ob = &mut op_builds.last_mut().unwrap().1;
                if key == "type" {
                    ob.op_type = OP_NAMES
                        .iter()
                        .position(|&name| name == val)
                        .map(|id| id as u8)
                        .unwrap_or_else(|| val.parse().unwrap_or(0));
                } else if key == "row_cache" {
                    ob.cache_size = val.parse().unwrap_or(0);
                } else if key.starts_with("input") {
                    ob.conns.push(parse_op_ref(val));
                } else if let Some(f) = parse_named_field(ob.op_type, key, val) {
                    ob.fields.push(f);
                } else if let Some(fi_str) = key.strip_prefix("field_") {
                    if let Ok(oc) = fi_str.parse::<u8>() {
                        ob.fields.push((oc, hex_decode(val)));
                    }
                }
            }
            Section::None => {}
        }
    }

    op_builds.sort_by_key(|(idx, _)| *idx);

    for (_, ob) in op_builds {
        let mut sprite_id = None;
        if ob.op_type == 39 || ob.op_type == 18 {
            if let Some(&(0, ref data)) = ob.fields.iter().find(|(oc, _)| *oc == 0) {
                if data.len() >= 2 {
                    sprite_id = Some(u16::from_be_bytes([data[0], data[1]]));
                }
            }
        }
        def.ops.push(TextureOp {
            op_type: ob.op_type,
            cache_size: ob.cache_size,
            fields: ob.fields,
            sprite_id,
        });
        def.connections.push(ob.conns);
    }

    def
}

#[cfg(since_647)]
fn hex_decode(s: &str) -> Vec<u8> {
    let s = s.trim();
    if s.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        out.push((hex_nibble(bytes[i]) << 4) | hex_nibble(bytes[i + 1]));
        i += 2;
    }
    out
}

#[cfg(since_647)]
fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

#[cfg(since_647)]
fn parse_op_ref(val: &str) -> u8 {
    val.strip_prefix("op").unwrap_or(val).parse().unwrap_or(0)
}

#[cfg(since_647)]
pub mod source;
