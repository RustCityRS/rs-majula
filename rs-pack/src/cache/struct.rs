use super::param::ParamType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rustc_hash::FxHashMap;

pub type StructTypeProvider = TypeProvider<StructType>;

pub struct StructType {
    pub id: u16,
    pub params: Option<Box<FxHashMap<i32, ParamValue>>>,
    debugname: Option<Box<str>>,
}

pub struct StructTypeRaw {
    pub id: u16,
    pub params: Option<Vec<(i32, ParamValue)>>,
    debugname: Option<Box<str>>,
}

impl CacheType for StructTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            params: None,
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
                249 => {
                    let count = buf.g1() as usize;
                    let params = self.params.get_or_insert_with(Vec::new);
                    params.reserve(count);
                    for _ in 0..count {
                        params.push(ParamType::decode_param(buf));
                    }
                }
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("struct", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

impl From<StructTypeRaw> for StructType {
    fn from(s: StructTypeRaw) -> Self {
        StructType {
            id: s.id,
            params: s
                .params
                .map(|params| Box::new(params.into_iter().collect())),
            debugname: s.debugname,
        }
    }
}

impl StructType {
    pub fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
