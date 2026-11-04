use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
pub use crate::types::VarPlayerScope;
use rs_io::Packet;

pub type VarPlayerTypeProvider = TypeProvider<VarPlayerType>;

pub struct VarPlayerType {
    pub id: u16,
    pub scope: VarPlayerScope,
    pub var_type: ScriptVarType,
    pub protect: bool,
    pub clientcode: u16,
    pub transmit: bool,
    pub(crate) debugname: Option<Box<str>>,
}

impl CacheType for VarPlayerType {
    type Context = ();

    fn new(id: u16) -> Self {
        VarPlayerType {
            id,
            scope: VarPlayerScope::Temp,
            var_type: ScriptVarType::Int,
            protect: true,
            clientcode: 0,
            transmit: false,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.scope = VarPlayerScope::try_from(buf.g1()).unwrap(),
                2 => self.var_type = ScriptVarType::try_from(buf.g1()).unwrap(),
                4 => self.protect = false,
                5 => self.clientcode = buf.g2(),
                6 => self.transmit = true,
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => crate::unknown_opcode!("varp", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
