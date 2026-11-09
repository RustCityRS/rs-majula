use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
use crate::types::VarPlayerScope;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type VarConTypeProvider = TypeProvider<VarConType>;

/// A controller var: its type, and whether it's kept past its controller.
pub struct VarConType {
    pub id: u16,
    pub var_type: ScriptVarType,
    pub scope: VarPlayerScope,
    debugname: Option<Box<str>>,
}

impl CacheType for VarConType {
    type Context = ();

    fn new(id: u16) -> Self {
        VarConType {
            id,
            var_type: ScriptVarType::Int,
            scope: VarPlayerScope::Temp,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            match code {
                1 => self.var_type = ScriptVarType::try_from(buf.g1()).unwrap(),
                2 => self.scope = VarPlayerScope::try_from(buf.g1()).unwrap(),
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("varcon", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
