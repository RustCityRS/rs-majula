use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type MesAnimTypeProvider = TypeProvider<MesAnimType>;

pub struct MesAnimType {
    pub id: u16,
    pub len: [Option<u16>; 4],
    debugname: Box<str>,
}

impl CacheType for MesAnimType {
    type Context = ();

    fn new(id: u16) -> Self {
        MesAnimType {
            id,
            len: [None; 4],
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
                1..=4 => self.len[code as usize - 1] = Some(buf.g2()),
                250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
                _ => crate::unknown_opcode!("mesanim", code),
            }
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}
