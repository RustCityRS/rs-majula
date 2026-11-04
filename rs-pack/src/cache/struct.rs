use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use rs_io::Packet;
use rustc_hash::FxHashMap;

pub type StructTypeProvider = TypeProvider<StructType>;

pub struct StructType {
    pub id: u16,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    pub(crate) debugname: Option<Box<str>>,
}

impl CacheType for StructType {
    type Context = ();

    fn new(id: u16) -> Self {
        StructType {
            id,
            params: None,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                249 => ParamType::decode_params(
                    buf,
                    self.params
                        .get_or_insert_with(|| Box::new(FxHashMap::default())),
                ),
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => crate::unknown_opcode!("struct", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
