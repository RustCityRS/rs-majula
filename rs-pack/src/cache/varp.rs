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
    pub debugname: Box<str>,
}

pub struct VarPlayerTypeRaw {
    pub id: u16,
    pub scope: VarPlayerScope,
    pub var_type: ScriptVarType,
    pub protect: bool,
    pub clientcode: u16,
    pub transmit: bool,
    debugname: Box<str>,
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
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.scope = VarPlayerScope::try_from(buf.g1()).unwrap(),
            2 => self.var_type = ScriptVarType::try_from(buf.g1()).unwrap(),
            4 => self.protect = false,
            5 => self.clientcode = buf.g2(),
            6 => self.transmit = true,
            250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
            _ => crate::unknown_opcode!("varp", code),
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
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
