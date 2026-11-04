//! FluType — CacheType implementation for floor underlay definitions.
//!
//! Decoded from archive 2, group 1 (Js5ConfigGroup.FLUTYPE).
//! Cross-referenced against rev-647 client `FluType.decode(Packet, int)`.

use rs_io::Packet;

use super::CacheType;

/// Floor underlay definition.
///
/// | op  | shape                                          |
/// |-----|------------------------------------------------|
/// | 1   | g3 (rgb color; client passes through rgb_to_hsl) |
/// | 2   | g2 (texture id; 65535 -> -1)                   |
/// | 3   | g2 (size base; client stores <<2)               |
/// | 4   | flag (clears aBool201)                         |
/// | 5   | flag (clears aBool202)                         |
/// | 250 | gjstr (debugname, server-side)                  |
pub struct FluType {
    pub id: u16,
    pub color: Option<u32>,
    pub texture: u16,
    pub has_texture: bool,
    pub size: u16,
    pub has_size: bool,
    pub op4: bool,
    pub op5: bool,
    debugname: Option<Box<str>>,
    /// Opcodes in decode order, for byte-perfect re-emission.
    pub opcode_order: Vec<u8>,
}

impl CacheType for FluType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            color: None,
            texture: 0,
            has_texture: false,
            size: 0,
            has_size: false,
            op4: false,
            op5: false,
            debugname: None,
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
                    self.texture = buf.g2();
                    self.has_texture = true;
                }
                3 => {
                    self.opcode_order.push(opcode);
                    self.size = buf.g2();
                    self.has_size = true;
                }
                4 => {
                    self.opcode_order.push(opcode);
                    self.op4 = true;
                }
                5 => {
                    self.opcode_order.push(opcode);
                    self.op5 = true;
                }
                250 => self.debugname = Some(buf.gjstr(0).into_boxed_str()),
                _ => {
                    buf.pos = buf.data.len();
                    return;
                }
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
