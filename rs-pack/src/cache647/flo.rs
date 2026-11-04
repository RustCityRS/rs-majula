//! FloType — CacheType implementation for floor overlay definitions.
//!
//! Decoded from archive 2, group 4 (Js5ConfigGroup.FLOTYPE).
//! Cross-referenced against rev-647 client `FloType.decode(Packet, int)`.

use rs_io::Packet;

use super::CacheType;

/// Floor overlay definition.
///
/// | op | shape                                             |
/// |----|---------------------------------------------------|
/// | 1  | g3 (tile color, raw RGB u24)                      |
/// | 2  | g1 (texture id u8)                                |
/// | 3  | g2 (texture id u16; 65535 -> -1)                  |
/// | 5  | flag (occlude = false)                            |
/// | 7  | g3 (water color, raw RGB u24)                     |
/// | 8  | flag (flolist mark)                               |
/// | 9  | g2 (client stores <<2)                            |
/// | 10 | flag (aBool34 = false)                            |
/// | 11 | g1 (anInt461)                                     |
/// | 12 | flag (aBool33 = true)                             |
/// | 13 | g3 (anInt452)                                     |
/// | 14 | g1 (client stores <<2)                            |
/// | 16 | g1 (anInt450)                                     |
pub struct FloType {
    pub id: u16,
    pub color: Option<u32>,
    pub texture: i32,
    pub occlude: bool,
    pub water_color: i32,
    pub op8_flomark: bool,
    pub op9_u16: Option<u16>,
    pub op10_bool34_false: bool,
    pub op11_values: Vec<u8>,
    pub op12_bool33_true: bool,
    pub op13_u24: Option<u32>,
    pub op14_u8: Option<u8>,
    pub op16_u8: Option<u8>,
    /// Tracks which texture opcode was used (2 = op2 u8, 3 = op3 u16).
    pub texture_opcode: u8,
    /// For op2, stores the raw u8 value.
    pub texture_u8_raw: u8,
    /// For op3, stores the raw u16 value.
    pub texture_u16_raw: u16,
    /// Opcodes in decode order, for byte-perfect re-emission.
    pub opcode_order: Vec<u8>,
}

impl CacheType for FloType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            color: None,
            texture: -1,
            occlude: true,
            water_color: -1,
            op8_flomark: false,
            op9_u16: None,
            op10_bool34_false: false,
            op11_values: Vec::new(),
            op12_bool33_true: false,
            op13_u24: None,
            op14_u8: None,
            op16_u8: None,
            texture_opcode: 0,
            texture_u8_raw: 0,
            texture_u16_raw: 0,
            opcode_order: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        loop {
            let opcode = buf.g1();
            if opcode == 0 {
                break;
            }
            match opcode {
                1 => {
                    self.opcode_order.push(opcode);
                    self.color = Some(buf.g3() as u32);
                }
                2 => {
                    self.opcode_order.push(opcode);
                    let v = buf.g1();
                    self.texture = v as i32;
                    self.texture_opcode = 2;
                    self.texture_u8_raw = v;
                }
                3 => {
                    self.opcode_order.push(opcode);
                    let v = buf.g2();
                    self.texture = if v == 65535 { -1 } else { v as i32 };
                    self.texture_opcode = 3;
                    self.texture_u16_raw = v;
                }
                5 => {
                    self.opcode_order.push(opcode);
                    self.occlude = false;
                }
                7 => {
                    self.opcode_order.push(opcode);
                    self.water_color = buf.g3();
                }
                8 => {
                    self.opcode_order.push(opcode);
                    self.op8_flomark = true;
                }
                9 => {
                    self.opcode_order.push(opcode);
                    self.op9_u16 = Some(buf.g2());
                }
                10 => {
                    self.opcode_order.push(opcode);
                    self.op10_bool34_false = true;
                }
                11 => {
                    self.opcode_order.push(opcode);
                    self.op11_values.push(buf.g1());
                }
                12 => {
                    self.opcode_order.push(opcode);
                    self.op12_bool33_true = true;
                }
                13 => {
                    self.opcode_order.push(opcode);
                    self.op13_u24 = Some(buf.g3() as u32);
                }
                14 => {
                    self.opcode_order.push(opcode);
                    self.op14_u8 = Some(buf.g1());
                }
                16 => {
                    self.opcode_order.push(opcode);
                    self.op16_u8 = Some(buf.g1());
                }
                _ => {
                    buf.pos = buf.data.len();
                    return;
                }
            }
        }
    }
}
