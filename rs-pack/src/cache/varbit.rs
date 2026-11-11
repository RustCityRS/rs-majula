use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type VarbitTypeProvider = TypeProvider<VarbitType>;

pub struct VarbitType {
    pub id: u16,
    pub basevar: u16,
    pub start_bit: u8,
    pub end_bit: u8,
    debugname: Box<str>,
}

impl CacheType for VarbitType {
    type Context = ();

    fn new(id: u16) -> Self {
        VarbitType {
            id,
            basevar: 0,
            start_bit: 0,
            end_bit: 0,
            debugname: Box::default(),
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
                    self.basevar = buf.g2();
                    self.start_bit = buf.g1();
                    self.end_bit = buf.g1();
                }
                250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
                _ => crate::unknown_opcode!("varpbit", code),
            }
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}
