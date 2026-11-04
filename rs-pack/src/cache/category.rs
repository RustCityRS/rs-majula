use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;

pub type CategoryTypeProvider = TypeProvider<CategoryType>;

pub struct CategoryType {
    pub id: u16,
    pub(crate) debugname: Option<Box<str>>,
}

impl CacheType for CategoryType {
    type Context = ();

    fn new(id: u16) -> Self {
        CategoryType {
            id,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => crate::unknown_opcode!("category", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
