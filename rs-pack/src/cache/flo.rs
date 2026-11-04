use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;

pub type FloTypeProvider = TypeProvider<FloType>;

pub struct FloType {
    pub id: u16,
    pub colour: i32,
    pub texture: Option<u8>,
    pub overlay: bool,
    pub occlude: bool,
    pub(crate) debugname: Option<Box<str>>,
}

impl CacheType for FloType {
    type Context = ();

    fn new(id: u16) -> Self {
        FloType {
            id,
            colour: 0,
            texture: None,
            overlay: false,
            occlude: true,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.colour = buf.g3(),
                2 => self.texture = Some(buf.g1()),
                3 => self.overlay = true,
                5 => self.occlude = false,
                6 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => crate::unknown_opcode!("flo", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
