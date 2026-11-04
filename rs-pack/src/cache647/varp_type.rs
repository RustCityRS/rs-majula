use rs_io::Packet;

use super::CacheType;

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize)]
pub enum VarPlayerScope {
    Temp = 0,
    Perm = 1,
}

impl VarPlayerScope {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Perm,
            _ => Self::Temp,
        }
    }
}

/// Archive 2, group 16 (VarPlayerType).
///
/// | op  | shape                                              |
/// |-----|----------------------------------------------------|
/// | 1   | g1 (scope: 0=temp, 1=perm)                         |
/// | 2   | g1 (var_type: ScriptVarType char)                  |
/// | 4   | flag (no payload)                                  |
/// | 5   | g2 (clientcode) — the only opcode the client reads |
/// | 6   | flag (transmit = true)                             |
/// | 250 | gjstr (debugname)                                  |
#[derive(Debug, Clone, serde::Serialize)]
pub struct VarpType {
    pub id: u16,
    pub scope: VarPlayerScope,
    pub var_type: Option<u8>,
    /// Opcode 4 flag — protect=false (server-only, client ignores).
    pub protect: bool,
    pub clientcode: u16,
    pub transmit: bool,
    debugname: Option<Box<str>>,
    /// Opcodes in decode order, for byte-perfect re-emission.
    pub opcode_order: Vec<u8>,
}

impl CacheType for VarpType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            scope: VarPlayerScope::Temp,
            var_type: None,
            protect: true,
            clientcode: 0,
            transmit: false,
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
                    self.scope = VarPlayerScope::from_u8(buf.g1());
                }
                2 => {
                    self.opcode_order.push(opcode);
                    self.var_type = Some(buf.g1());
                }
                4 => {
                    self.opcode_order.push(opcode);
                    self.protect = false;
                }
                5 => {
                    self.opcode_order.push(opcode);
                    self.clientcode = buf.g2();
                }
                6 => {
                    self.opcode_order.push(opcode);
                    self.transmit = true;
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
