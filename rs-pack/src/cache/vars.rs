use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;

pub type VarsTypeProvider = TypeProvider<VarsType>;

pub struct VarsType {
    pub id: u16,
    pub var_type: ScriptVarType,
    pub(crate) debugname: Option<Box<str>>,
}

impl CacheType for VarsType {
    type Context = ();

    fn new(id: u16) -> Self {
        VarsType {
            id,
            var_type: ScriptVarType::Int,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.var_type = ScriptVarType::try_from(buf.g1()).unwrap(),
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => crate::unknown_opcode!("vars", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
