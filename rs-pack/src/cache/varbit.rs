#[cfg(since_254)]
use super::provider::{CacheType, TypeProvider};
#[cfg(since_254)]
use rs_io::Packet;

#[cfg(since_254)]
pub type VarbitTypeProvider = TypeProvider<VarbitType>;

#[cfg(since_254)]
pub struct VarbitType {
    pub id: u16,
    pub basevar: u16,
    pub start_bit: u8,
    pub end_bit: u8,
    debugname: Option<Box<str>>,
}

#[cfg(since_254)]
impl CacheType for VarbitType {
    type Context = ();

    fn new(id: u16) -> Self {
        VarbitType {
            id,
            basevar: 0,
            start_bit: 0,
            end_bit: 0,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => {
                    self.basevar = buf.g2();
                    self.start_bit = buf.g1();
                    self.end_bit = buf.g1();
                }
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => panic!("Unrecognized varbit config code: {code}"),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
