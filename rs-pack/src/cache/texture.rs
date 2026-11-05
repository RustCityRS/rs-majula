#[derive(Debug, Clone)]
pub struct TextureDef {
    pub ops: Vec<TextureOp>,
    pub connections: Vec<Vec<u8>>,
    pub color_op: u8,
    pub alpha_op: u8,
    pub mod_op: u8,
    pub trailing_material: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct TextureOp {
    pub op_type: u8,
    pub cache_size: u8,
    pub fields: Vec<(u8, Vec<u8>)>,
    pub sprite_id: Option<u16>,
}

impl TextureDef {
    pub fn primary_sprite(&self) -> Option<u16> {
        self.ops.iter().find_map(|op| op.sprite_id)
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
            0 | 1 => {
                skip(data, pos, 1);
            }
            2..=7 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        5 => {
            if let 0..=2 = opcode {
                skip(data, pos, 1);
            }
        }
        6 => match opcode {
            0 | 1 => {
                skip(data, pos, 2);
            }
            2 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        7 => match opcode {
            0 | 1 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        8 => {
            if opcode == 0 {
                skip(data, pos, 1); // mode
                let count = read_u8(data, pos) as usize;
                skip(data, pos, count * 4); // markers: g2+g2 each
            }
        }
        9 => {
            if let 0..=2 = opcode {
                skip(data, pos, 1);
            }
        }
        10 => {
            if opcode == 0 {
                let preset = read_u8(data, pos);
                if preset == 0 {
                    let count = read_u8(data, pos) as usize;
                    skip(data, pos, count * 5); // g2 + g1 + g1 + g1
                }
            }
        }
        11 => {
            if let 0..=2 = opcode {
                skip(data, pos, 2);
            }
        }
        12 => match opcode {
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
            0 | 1 | 3 | 4 | 5 | 6 => {
                skip(data, pos, 1);
            }
            2 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        16 => match opcode {
            0 | 1 => {
                skip(data, pos, 1);
            }
            2 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        17 => match opcode {
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
            0 => {
                skip(data, pos, 2);
            }
            1 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        20 => match opcode {
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
            0..=3 => {
                skip(data, pos, 2);
            }
            4 => {
                skip(data, pos, 3);
            }
            _ => {}
        },
        26 => match opcode {
            0 | 1 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        27 => match opcode {
            0 | 2 => {
                skip(data, pos, 1);
            }
            1 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        28 => match opcode {
            0 | 6 => {
                skip(data, pos, 1);
            }
            1 | 2 | 3 | 4 | 5 | 7 | 8 => {
                skip(data, pos, 2);
            }
            _ => {}
        },
        29 => match opcode {
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
            0 | 1 => {
                skip(data, pos, 2);
            }
            2 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        31 => {
            if let 0..=3 = opcode {
                skip(data, pos, 2);
            }
        }
        32 => {
            if let 0..=2 = opcode {
                skip(data, pos, 2);
            }
        }
        33 => match opcode {
            0 => {
                skip(data, pos, 2);
            }
            1 => {
                skip(data, pos, 1);
            }
            _ => {}
        },
        34 => match opcode {
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
        37 => {
            if let 0..=6 = opcode {
                skip(data, pos, 2);
            }
        }
        38 => match opcode {
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
            fields,
            sprite_id,
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

pub(crate) const OP_NAMES: [&str; 40] = [
    "constant_mono",
    "constant_color",
    "horizontal_gradient",
    "vertical_gradient",
    "checkerboard",
    "blur",
    "clamp",
    "arithmetic",
    "curve",
    "mirror",
    "gradient_map",
    "color_strip",
    "diagonal_gradient",
    "white_noise",
    "diamond",
    "voronoi",
    "weave",
    "hsl_adjust",
    "wrapped_sprite",
    "trig_warp",
    "tiling",
    "lerp",
    "invert",
    "polar_coords",
    "color_to_mono",
    "color_replace",
    "binary_threshold",
    "square_wave",
    "bricks",
    "vector_graphics",
    "brightness",
    "mandelbrot",
    "emboss",
    "normal_map",
    "perlin_noise",
    "edge_detect",
    "texture_ref",
    "plaid",
    "scatter_lines",
    "sprite",
];
