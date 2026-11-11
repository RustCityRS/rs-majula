use super::provider::{CacheType, TypeProvider};
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type CategoryTypeProvider = TypeProvider<CategoryType>;

pub struct CategoryType {
    pub id: u16,
    debugname: Box<str>,
}

impl CacheType for CategoryType {
    type Context = ();

    fn new(id: u16) -> Self {
        CategoryType {
            id,
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
                1 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
                _ => crate::unknown_opcode!("category", code),
            }
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}
