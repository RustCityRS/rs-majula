//! SpotAnimType — CacheType implementation for spot animation (graphic effect) definitions.
//!
//! Decoded from archive 21, strided u16 ids (stride 256).
//! Cross-referenced against rev-647 client `SpotAnimType.decode(Packet, int)`.

use rs_io::Packet;

use super::CacheType;

/// Spot animation definition.
///
/// | op | shape                                                  |
/// |----|--------------------------------------------------------|
/// | 1  | g2 (model id)                                          |
/// | 2  | g2 (seq/anim id)                                       |
/// | 4  | g2 (resize x)                                          |
/// | 5  | g2 (resize z)                                          |
/// | 6  | g2 (rotation)                                          |
/// | 7  | g1 (ambient)                                           |
/// | 8  | g1 (contrast)                                          |
/// | 9  | flag (anim_mode=3, anim_param=8224)                    |
/// | 10 | flag (aBool203 = true)                                 |
/// | 11 | flag (anim_mode=1)                                     |
/// | 12 | flag (anim_mode=4)                                     |
/// | 13 | flag (anim_mode=5)                                     |
/// | 14 | g1 (anim_mode=2, anim_param = arg*256)                 |
/// | 15 | g2 (anim_mode=3, anim_param = u16)                     |
/// | 16 | g4s (anim_mode=3, anim_param = i32)                    |
/// | 40 | g1 count, count x (g2 dst, g2 src) — recolor           |
/// | 41 | g1 count, count x (g2 dst, g2 src) — retexture         |
pub struct SpotAnimType {
    pub id: u16,
    pub model: u16,
    pub has_model: bool,
    pub seq: u16,
    pub has_seq: bool,
    pub resize_x: u16,
    pub resize_z: u16,
    pub rotation: u16,
    pub ambient: u8,
    pub contrast: u8,
    pub op9: bool,
    pub op10: bool,
    pub op11: bool,
    pub op12: bool,
    pub op13: bool,
    pub op14: u8,
    pub has_op14: bool,
    pub op15: u16,
    pub has_op15: bool,
    pub op16: i32,
    pub has_op16: bool,
    pub recol_d: Vec<u16>,
    pub recol_s: Vec<u16>,
    pub retex_d: Vec<u16>,
    pub retex_s: Vec<u16>,
    /// Opcodes in decode order, for byte-perfect re-emission.
    pub opcode_order: Vec<u8>,
}

impl CacheType for SpotAnimType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            model: 0,
            has_model: false,
            seq: 0,
            has_seq: false,
            resize_x: 128,
            resize_z: 128,
            rotation: 0,
            ambient: 0,
            contrast: 0,
            op9: false,
            op10: false,
            op11: false,
            op12: false,
            op13: false,
            op14: 0,
            has_op14: false,
            op15: 0,
            has_op15: false,
            op16: 0,
            has_op16: false,
            recol_d: Vec::new(),
            recol_s: Vec::new(),
            retex_d: Vec::new(),
            retex_s: Vec::new(),
            opcode_order: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        loop {
            let opcode = buf.g1();
            if opcode == 0 {
                break;
            }
            self.opcode_order.push(opcode);
            match opcode {
                1 => {
                    self.model = buf.g2();
                    self.has_model = true;
                }
                2 => {
                    self.seq = buf.g2();
                    self.has_seq = true;
                }
                4 => self.resize_x = buf.g2(),
                5 => self.resize_z = buf.g2(),
                6 => self.rotation = buf.g2(),
                7 => self.ambient = buf.g1(),
                8 => self.contrast = buf.g1(),
                9 => self.op9 = true,
                10 => self.op10 = true,
                11 => self.op11 = true,
                12 => self.op12 = true,
                13 => self.op13 = true,
                14 => {
                    self.op14 = buf.g1();
                    self.has_op14 = true;
                }
                15 => {
                    self.op15 = buf.g2();
                    self.has_op15 = true;
                }
                16 => {
                    self.op16 = buf.g4s();
                    self.has_op16 = true;
                }
                40 => {
                    let count = buf.g1() as usize;
                    self.recol_d = Vec::with_capacity(count);
                    self.recol_s = Vec::with_capacity(count);
                    for _ in 0..count {
                        self.recol_d.push(buf.g2());
                        self.recol_s.push(buf.g2());
                    }
                }
                41 => {
                    let count = buf.g1() as usize;
                    self.retex_d = Vec::with_capacity(count);
                    self.retex_s = Vec::with_capacity(count);
                    for _ in 0..count {
                        self.retex_d.push(buf.g2());
                        self.retex_s.push(buf.g2());
                    }
                }
                _ => {
                    buf.pos = buf.data.len();
                    return;
                }
            }
        }
    }
}
