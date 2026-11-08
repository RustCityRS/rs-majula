use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type VarObjTypeProvider = TypeProvider<VarObjType>;

/// Bits `start_bit..=end_bit` of the 32 an obj carries for its varobjs.
pub struct VarObjType {
    pub id: u16,
    pub start_bit: u8,
    pub end_bit: u8,
    debugname: Option<Box<str>>,
}

impl CacheType for VarObjType {
    type Context = ();

    fn new(id: u16) -> Self {
        VarObjType {
            id,
            start_bit: 0,
            end_bit: 0,
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
                1 => {
                    self.start_bit = buf.g1();
                    self.end_bit = buf.g1();
                }
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("varobj", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
