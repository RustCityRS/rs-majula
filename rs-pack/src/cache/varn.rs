use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type VarnTypeProvider = TypeProvider<VarnType>;

pub struct VarnType {
    pub id: u16,
    pub var_type: ScriptVarType,
    debugname: Option<Box<str>>,
}

impl CacheType for VarnType {
    type Context = ();

    fn new(id: u16) -> Self {
        VarnType {
            id,
            var_type: ScriptVarType::Int,
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
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("varn", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
