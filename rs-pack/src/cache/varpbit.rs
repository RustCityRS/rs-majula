use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;

pub type VarpBitTypeProvider = TypeProvider<VarpBitType>;

pub struct VarpBitType {
    pub id: TypeId,
    pub basevar: u16,
    pub start_bit: u8,
    pub end_bit: u8,
    debugname: Box<str>,
}

impl CacheType for VarpBitType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        VarpBitType {
            id,
            basevar: 0,
            start_bit: 0,
            end_bit: 0,
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
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

    fn debugname(&self) -> &str {
        &self.debugname
    }
}
