use rs_io::Packet;

use super::CacheType;

/// Archive 2, group 3 (Js5ConfigGroup.IDKTYPE).
///
/// | op      | shape                                              |
/// |---------|----------------------------------------------------|
/// | 1       | g1 (body part type)                                |
/// | 2       | g1 count, count x g2 (model ids)                   |
/// | 3       | flag (disable / non-selectable)                    |
/// | 40      | g1 count, count x (g2 src, g2 dst) — recolor pairs |
/// | 41      | g1 count, count x (g2 src, g2 dst) — retexture     |
/// | 60..=69 | g2 — head model id at slot (opcode - 60)           |
pub struct IdkType {
    pub id: u16,
    pub body_type: Option<u8>,
    pub disable: bool,
    pub models: Vec<u16>,
    pub recol_s: Vec<u16>,
    pub recol_d: Vec<u16>,
    pub retex_s: Vec<u16>,
    pub retex_d: Vec<u16>,
    /// Head model ids, 10 slots. -1 = unused.
    pub heads: [i32; 10],
    /// Opcodes in decode order, for byte-perfect re-emission.
    pub opcode_order: Vec<u8>,
}

impl CacheType for IdkType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            body_type: None,
            disable: false,
            models: Vec::new(),
            recol_s: Vec::new(),
            recol_d: Vec::new(),
            retex_s: Vec::new(),
            retex_d: Vec::new(),
            heads: [-1; 10],
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
                    self.body_type = Some(buf.g1());
                }
                2 => {
                    self.opcode_order.push(opcode);
                    let count = buf.g1() as usize;
                    self.models = Vec::with_capacity(count);
                    for _ in 0..count {
                        self.models.push(buf.g2());
                    }
                }
                3 => {
                    self.opcode_order.push(opcode);
                    self.disable = true;
                }
                40 => {
                    self.opcode_order.push(opcode);
                    let count = buf.g1() as usize;
                    self.recol_s = Vec::with_capacity(count);
                    self.recol_d = Vec::with_capacity(count);
                    for _ in 0..count {
                        self.recol_s.push(buf.g2());
                        self.recol_d.push(buf.g2());
                    }
                }
                41 => {
                    self.opcode_order.push(opcode);
                    let count = buf.g1() as usize;
                    self.retex_s = Vec::with_capacity(count);
                    self.retex_d = Vec::with_capacity(count);
                    for _ in 0..count {
                        self.retex_s.push(buf.g2());
                        self.retex_d.push(buf.g2());
                    }
                }
                60..=69 => {
                    self.opcode_order.push(opcode);
                    let slot = (opcode - 60) as usize;
                    self.heads[slot] = buf.g2() as i32;
                }
                _ => {
                    buf.pos = buf.data.len();
                    return;
                }
            }
        }
    }
}
