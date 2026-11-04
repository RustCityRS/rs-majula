#![allow(clippy::single_match)]
// Ported verbatim from the rev-647 source repo; style lints allowed to keep diffs clean.
//! Texture archive format: procedural texture DAG parser + encoder + text format.
//!
//! Archive 9 (TEXTURES) stores procedural texture definitions as directed
//! acyclic graphs (DAGs) of TextureOp operations. Each group is one texture.
//! The binary is decoded from the rev-647 client's `Texture(Packet)` constructor.
//!
//! Pipeline: binary ↔ TextureDef ↔ text (.texture files).

use serde::{Deserialize, Serialize};

pub const TEXTURE_ARCHIVE: u8 = 9;

// ── Manifest ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureManifest {
    pub archive_id: u8,
    pub groups: Vec<TextureGroupEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextureGroupEntry {
    pub group_id: u32,
    pub file_name: String,
    pub compression: u8,
    pub version: u16,
    pub compressed_len: u32,
    pub decompressed_len: u32,
    pub crc: u32,
}

// ── Types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TextureDef {
    pub ops: Vec<TextureOp>,
    pub connections: Vec<Vec<u8>>,
    pub color_op: u8,
    pub alpha_op: u8,
    pub mod_op: u8,
    pub consumed: usize,
    /// 10-byte trailing material metadata embedded in the texture binary.
    /// Duplicates data from archive 26 (materials). The client ignores these
    /// bytes (Texture constructor stops before them), but they must be
    /// preserved for byte-perfect repack.
    pub trailing_material: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TextureOp {
    pub op_type: u8,
    pub cache_size: u8,
    pub input_count: u8,
    pub fields: Vec<(u8, Vec<u8>)>,
    pub sprite_id: Option<u16>,
    pub texture_ref: Option<u16>,
}

impl TextureDef {
    pub fn sprite_ids(&self) -> Vec<u16> {
        self.ops.iter().filter_map(|op| op.sprite_id).collect()
    }

    pub fn primary_sprite(&self) -> Option<u16> {
        self.sprite_ids().into_iter().next()
    }
}

fn input_count_for_type(op_type: u8) -> u8 {
    match op_type {
        0 | 1 | 2 | 3 | 4 | 12 | 13 | 14 | 15 | 16 | 27 | 28 | 29 | 31 | 34 | 36 | 37 | 38 | 39
        | 18 => 0,
        5 | 6 | 8 | 9 | 10 | 11 | 17 | 20 | 22 | 23 | 24 | 25 | 26 | 30 | 32 | 33 | 35 => 1,
        7 => 2,
        19 | 21 => 3,
        _ => 0,
    }
}

fn read_field_data(
    data: &[u8],
    pos: &mut usize,
    op_type: u8,
    opcode: u8,
    ctx_count: u8,
) -> Vec<u8> {
    let start = *pos;

    match op_type {
        0 => {
            if opcode == 0 {
                skip(data, pos, 1);
            }
        }
        1 => {
            if opcode == 0 {
                skip(data, pos, 3);
            }
        }
        2 | 3 | 13 => {}
        4 => match opcode {
            // Sub24
            0 | 1 => {
                skip(data, pos, 1);
            }
            2..=7 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        5 => match opcode {
            // Sub11
            0..=2 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        6 => match opcode {
            // Sub5
            0 | 1 => {
                skip(data, pos, 2);
            }
            2 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        7 => match opcode {
            // Arithmetic
            0 | 1 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        8 => match opcode {
            // CurveTextureOp
            0 => {
                skip(data, pos, 1); // mode
                let count = read_u8(data, pos) as usize;
                skip(data, pos, count * 4); // markers: g2+g2 each
            }
            _ => {}
        },
        9 => match opcode {
            // Mirror
            0..=2 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        10 => match opcode {
            // GradientTextureOp
            0 => {
                let preset = read_u8(data, pos);
                if preset == 0 {
                    let count = read_u8(data, pos) as usize;
                    skip(data, pos, count * 5); // g2 + g1 + g1 + g1
                }
            }
            _ => {}
        },
        11 => match opcode {
            // ColorStrip
            0..=2 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        12 => match opcode {
            // DiagonalGradient
            0 | 1 | 3 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        14 => {
            if opcode == 0 {
                skip(data, pos, 2);
            }
        } // Sub38
        15 => match opcode {
            // Sub26
            0 | 1 | 3 | 4 | 5 | 6 => {
                skip(data, pos, 1);
            }
            2 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        16 => match opcode {
            // Sub15
            0 | 1 => {
                skip(data, pos, 1);
            }
            2 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        17 => match opcode {
            // Sub7
            0 => {
                skip(data, pos, 2);
            }
            1 | 2 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        18 | 39 => {
            if opcode == 0 {
                skip(data, pos, 2);
            }
        } // ImageSource
        19 => match opcode {
            // TrigWrap
            0 => {
                skip(data, pos, 2);
            }
            1 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        20 => match opcode {
            // Tiling
            0 | 1 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        21 => {
            if opcode == 0 {
                skip(data, pos, 1);
            }
        } // Sub35
        22 => {
            if opcode == 0 {
                skip(data, pos, 1);
            }
        } // Sub3
        23 => {
            if opcode == 0 {
                skip(data, pos, 1);
            }
        } // Sub23
        24 => {} // Sub39: no fields
        25 => match opcode {
            // Sub27
            0..=3 => {
                skip(data, pos, 2);
            }
            4 => {
                skip(data, pos, 3);
            }
            _ => {}
        },
        26 => match opcode {
            // Sub29
            0 | 1 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        27 => match opcode {
            // Sub21
            0 | 2 => {
                skip(data, pos, 1);
            }
            1 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        28 => match opcode {
            // Sub4
            0 | 6 => {
                skip(data, pos, 1);
            }
            1 | 2 | 3 | 4 | 5 | 7 | 8 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        29 => match opcode {
            // Sub30
            0 => {
                let count = read_u8(data, pos) as usize;
                for _ in 0..count {
                    let subtype = read_u8(data, pos);
                    match subtype {
                        0 => {
                            skip(data, pos, 12);
                        }
                        1 => {
                            skip(data, pos, 20);
                        }
                        2 | 3 => {
                            skip(data, pos, 15);
                        }
                        _ => {}
                    }
                }
            }
            1 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        30 => match opcode {
            // Sub16
            0 | 1 => {
                skip(data, pos, 2);
            }
            2 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        31 => match opcode {
            // Sub12
            0..=3 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        32 => match opcode {
            // Sub22
            0..=2 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        33 => match opcode {
            // Sub28
            0 => {
                skip(data, pos, 2);
            }
            1 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        34 => match opcode {
            // Sub10
            0 | 1 | 3 | 4 | 5 | 6 => {
                skip(data, pos, 1);
            }
            2 => {
                let val = read_i16_be(data, pos);
                if val < 0 {
                    skip(data, pos, ctx_count as usize * 2);
                }
            }
            _ => {}
        },
        35 => {
            if opcode == 0 {
                skip(data, pos, 2);
            }
        } // Sub32
        36 => {
            if opcode == 0 {
                skip(data, pos, 2);
            }
        } // Sub33 (texture ref)
        37 => match opcode {
            // Sub8
            0..=6 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        38 => match opcode {
            // Sub34
            0 | 2 => {
                skip(data, pos, 1);
            }
            1 | 3 | 4 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        _ => {}
    }

    let end = (*pos).min(data.len());
    let start = start.min(end);
    data[start..end].to_vec()
}

pub fn parse_texture(data: &[u8]) -> TextureDef {
    let mut pos = 0;
    let op_count = read_u8(data, &mut pos) as usize;

    let mut ops = Vec::with_capacity(op_count);
    let mut connections = Vec::with_capacity(op_count);
    let mut sub10_count: u8 = 4; // TextureOp_Sub10.anInt5433 defaults to 4

    for _ in 0..op_count {
        let _padding = read_u8(data, &mut pos);
        let op_type = read_u8(data, &mut pos);
        let cache_size = read_u8(data, &mut pos);
        let field_count = read_u8(data, &mut pos) as usize;

        let input_count = input_count_for_type(op_type);
        let mut sprite_id = None;
        let mut texture_ref = None;
        let mut fields = Vec::with_capacity(field_count);

        if op_type == 34 {
            sub10_count = 4;
        }

        for _ in 0..field_count {
            let opcode = read_u8(data, &mut pos);

            if op_type == 34 && opcode == 1 {
                sub10_count = data.get(pos).copied().unwrap_or(0);
            }

            let field_data = read_field_data(data, &mut pos, op_type, opcode, sub10_count);
            if (op_type == 39 || op_type == 18) && opcode == 0 && field_data.len() >= 2 {
                sprite_id = Some(u16::from_be_bytes([field_data[0], field_data[1]]));
            }
            if op_type == 36 && opcode == 0 && field_data.len() >= 2 {
                texture_ref = Some(u16::from_be_bytes([field_data[0], field_data[1]]));
            }
            fields.push((opcode, field_data));
        }

        let mut conns = Vec::with_capacity(input_count as usize);
        for _ in 0..input_count {
            conns.push(read_u8(data, &mut pos));
        }
        connections.push(conns);

        ops.push(TextureOp {
            op_type,
            cache_size,
            input_count,
            fields,
            sprite_id,
            texture_ref,
        });
    }

    let color_op = read_u8(data, &mut pos);
    let alpha_op = read_u8(data, &mut pos);
    let mod_op = read_u8(data, &mut pos);

    let trailing_material = data[pos..].to_vec();

    TextureDef {
        ops,
        connections,
        color_op,
        alpha_op,
        mod_op,
        consumed: pos,
        trailing_material,
    }
}

fn read_u8(data: &[u8], pos: &mut usize) -> u8 {
    let v = data.get(*pos).copied().unwrap_or(0);
    *pos += 1;
    v
}

fn read_i16_be(data: &[u8], pos: &mut usize) -> i16 {
    let hi = data.get(*pos).copied().unwrap_or(0) as u16;
    let lo = data.get(*pos + 1).copied().unwrap_or(0) as u16;
    *pos += 2;
    ((hi << 8) | lo) as i16
}

fn skip(data: &[u8], pos: &mut usize, n: usize) {
    *pos = (*pos + n).min(data.len());
}

// ── Structured → binary ──────────────────────────────────────────────────

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

// ── Op type names ────────────────────────────────────────────────────────

pub fn op_type_name(t: u8) -> &'static str {
    match t {
        0 => "constant_mono",
        1 => "constant_color",
        2 => "horizontal_gradient",
        3 => "vertical_gradient",
        4 => "checkerboard",
        5 => "blur",
        6 => "clamp",
        7 => "arithmetic",
        8 => "curve",
        9 => "mirror",
        10 => "gradient_map",
        11 => "color_strip",
        12 => "diagonal_gradient",
        13 => "white_noise",
        14 => "diamond",
        15 => "voronoi",
        16 => "weave",
        17 => "hsl_adjust",
        18 => "wrapped_sprite",
        19 => "trig_warp",
        20 => "tiling",
        21 => "lerp",
        22 => "invert",
        23 => "polar_coords",
        24 => "color_to_mono",
        25 => "color_replace",
        26 => "binary_threshold",
        27 => "square_wave",
        28 => "bricks",
        29 => "vector_graphics",
        30 => "brightness",
        31 => "mandelbrot",
        32 => "emboss",
        33 => "normal_map",
        34 => "perlin_noise",
        35 => "edge_detect",
        36 => "texture_ref",
        37 => "plaid",
        38 => "scatter_lines",
        39 => "sprite",
        _ => "unknown",
    }
}

fn parse_type_name(s: &str) -> u8 {
    match s {
        "constant_mono" => 0,
        "constant_color" => 1,
        "horizontal_gradient" => 2,
        "vertical_gradient" => 3,
        "checkerboard" => 4,
        "blur" => 5,
        "clamp" => 6,
        "arithmetic" => 7,
        "curve" => 8,
        "mirror" => 9,
        "gradient_map" => 10,
        "color_strip" => 11,
        "diagonal_gradient" => 12,
        "white_noise" => 13,
        "diamond" => 14,
        "voronoi" => 15,
        "weave" => 16,
        "hsl_adjust" => 17,
        "wrapped_sprite" => 18,
        "trig_warp" => 19,
        "tiling" => 20,
        "lerp" => 21,
        "invert" => 22,
        "polar_coords" => 23,
        "color_to_mono" => 24,
        "color_replace" => 25,
        "binary_threshold" => 26,
        "square_wave" => 27,
        "bricks" => 28,
        "vector_graphics" => 29,
        "brightness" => 30,
        "mandelbrot" => 31,
        "emboss" => 32,
        "normal_map" => 33,
        "perlin_noise" => 34,
        "edge_detect" => 35,
        "texture_ref" => 36,
        "plaid" => 37,
        "scatter_lines" => 38,
        "sprite" => 39,
        other => other.parse().unwrap_or(0),
    }
}

// ── Byte read/write helpers ─────────────────────────────────────────────

fn r_u8(d: &[u8]) -> u8 {
    d.first().copied().unwrap_or(0)
}
fn r_u16(d: &[u8]) -> u16 {
    u16::from_be_bytes([
        d.first().copied().unwrap_or(0),
        d.get(1).copied().unwrap_or(0),
    ])
}
fn r_i16(d: &[u8]) -> i16 {
    r_u16(d) as i16
}
fn r_u24(d: &[u8]) -> u32 {
    ((d.first().copied().unwrap_or(0) as u32) << 16)
        | ((d.get(1).copied().unwrap_or(0) as u32) << 8)
        | (d.get(2).copied().unwrap_or(0) as u32)
}
fn w_u8(v: u8) -> Vec<u8> {
    vec![v]
}
fn w_u16(v: u16) -> Vec<u8> {
    v.to_be_bytes().to_vec()
}
fn w_i16(v: i16) -> Vec<u8> {
    (v as u16).to_be_bytes().to_vec()
}
fn w_u24(v: u32) -> Vec<u8> {
    vec![(v >> 16) as u8, (v >> 8) as u8, v as u8]
}

// ── Named field emit ────────────────────────────────────────────────────

fn emit_field(op: u8, oc: u8, d: &[u8]) -> String {
    match (op, oc) {
        (0, 0) => format!("value={}", r_u8(d)),
        (1, 0) => format!("color=0x{:06x}", r_u24(d)),
        // 2, 3: no fields
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
        // 13: no fields
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
        // 24: no fields
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

// ── Named field parse ───────────────────────────────────────────────────

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

// ── Text emitter ────────────────────────────────────────────────────────

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
        out.push_str(&format!("type={}\n", op_type_name(op.op_type)));
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

// ── Text parser ─────────────────────────────────────────────────────────

pub fn parse_texture_text(text: &str) -> TextureDef {
    let mut def = TextureDef {
        ops: Vec::new(),
        connections: Vec::new(),
        color_op: 0,
        alpha_op: 0,
        mod_op: 0,
        consumed: 0,
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
                    ob.op_type = parse_type_name(val);
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
        let input_count = input_count_for_type(ob.op_type);
        let mut sprite_id = None;
        let mut texture_ref = None;
        if ob.op_type == 39 || ob.op_type == 18 {
            if let Some(&(0, ref data)) = ob.fields.iter().find(|(oc, _)| *oc == 0) {
                if data.len() >= 2 {
                    sprite_id = Some(u16::from_be_bytes([data[0], data[1]]));
                }
            }
        }
        if ob.op_type == 36 {
            if let Some(&(0, ref data)) = ob.fields.iter().find(|(oc, _)| *oc == 0) {
                if data.len() >= 2 {
                    texture_ref = Some(u16::from_be_bytes([data[0], data[1]]));
                }
            }
        }

        def.ops.push(TextureOp {
            op_type: ob.op_type,
            cache_size: ob.cache_size,
            input_count,
            fields: ob.fields,
            sprite_id,
            texture_ref,
        });
        def.connections.push(ob.conns);
    }

    def
}

fn hex_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for &b in data {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

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

fn hex_nibble(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => b - b'a' + 10,
        b'A'..=b'F' => b - b'A' + 10,
        _ => 0,
    }
}

fn parse_op_ref(val: &str) -> u8 {
    val.strip_prefix("op").unwrap_or(val).parse().unwrap_or(0)
}
