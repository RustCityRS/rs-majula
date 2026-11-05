use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
pub use crate::types::VarPlayerScope;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type VarPlayerTypeProvider = TypeProvider<VarPlayerType>;

pub struct VarPlayerType {
    pub id: u16,
    pub scope: VarPlayerScope,
    pub var_type: ScriptVarType,
    pub protect: bool,
    pub clientcode: u16,
    pub transmit: bool,
    debugname: Option<Box<str>>,
}

pub struct VarPlayerTypeRaw {
    pub id: u16,
    pub scope: VarPlayerScope,
    pub var_type: ScriptVarType,
    pub protect: bool,
    pub clientcode: u16,
    pub transmit: bool,
    debugname: Option<Box<str>>,
    pub opcode_order: Vec<u8>,
}

impl CacheType for VarPlayerTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            scope: VarPlayerScope::Temp,
            var_type: ScriptVarType::Int,
            protect: true,
            clientcode: 0,
            transmit: false,
            debugname: None,
            opcode_order: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            self.opcode_order.push(code);
            match code {
                1 => self.scope = VarPlayerScope::try_from(buf.g1()).unwrap(),
                2 => self.var_type = ScriptVarType::try_from(buf.g1()).unwrap(),
                4 => self.protect = false,
                5 => self.clientcode = buf.g2(),
                6 => self.transmit = true,
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("varp", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

impl From<VarPlayerTypeRaw> for VarPlayerType {
    fn from(v: VarPlayerTypeRaw) -> Self {
        VarPlayerType {
            id: v.id,
            scope: v.scope,
            var_type: v.var_type,
            protect: v.protect,
            clientcode: v.clientcode,
            transmit: v.transmit,
            debugname: v.debugname,
        }
    }
}

impl VarPlayerType {
    pub fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
