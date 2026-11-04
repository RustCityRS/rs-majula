use super::provider::CacheType;
use rs_io::Packet;

pub struct MesAnimType {
    pub id: u16,
    pub len: [Option<u16>; 4],
    debugname: Option<Box<str>>,
}

impl CacheType for MesAnimType {
    type Context = ();

    fn new(id: u16) -> Self {
        MesAnimType {
            id,
            len: [None; 4],
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            match code {
                0 => break,
                1..=4 => self.len[code as usize - 1] = Some(buf.g2()),
                250 => self.debugname = Some(buf.gjstr(0).into_boxed_str()),
                _ => {}
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
