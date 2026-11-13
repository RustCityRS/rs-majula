use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type VarsTypeProvider = TypeProvider<VarsType>;

pub struct VarsType {
    pub id: u16,
    pub var_type: ScriptVarType,
    pub(crate) debugname: Box<str>,
}

impl CacheType for VarsType {
    type Context = ();

    fn new(id: u16) -> Self {
        VarsType {
            id,
            var_type: ScriptVarType::Int,
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => self.var_type = ScriptVarType::try_from(buf.g1()).unwrap(),
            250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
            _ => crate::unknown_opcode!("vars", code),
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}
