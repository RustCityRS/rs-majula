use super::provider::{CacheType, TypeProvider};
use crate::types::ParamValue;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;

pub type ControllerTypeProvider = TypeProvider<ControllerType>;

/// A controller type: the values its configs give its varcons.
pub struct ControllerType {
    pub id: TypeId,
    pub vars: Box<[(u16, ParamValue)]>,
    pub varpbits: Box<[(u16, i32)]>,
    debugname: Box<str>,
}

impl CacheType for ControllerType {
    type Context = ();

    fn new(id: TypeId) -> Self {
        ControllerType {
            id,
            vars: Box::new([]),
            varpbits: Box::new([]),
            debugname: Box::default(),
        }
    }

    fn decode_opcode(&mut self, buf: &mut Packet, code: u8) {
        match code {
            1 => {
                let count = buf.g1() as usize;
                let mut vars = Vec::with_capacity(count);
                for _ in 0..count {
                    let id = buf.g2();
                    let value = if buf.g1() == 1 {
                        ParamValue::String(buf.gjstr(STRING_TERMINATOR).into_boxed_str())
                    } else {
                        ParamValue::Int(buf.g4s())
                    };
                    vars.push((id, value));
                }
                self.vars = vars.into_boxed_slice();
            }
            2 => {
                let count = buf.g1() as usize;
                self.varpbits = (0..count).map(|_| (buf.g2(), buf.g4s())).collect();
            }
            250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
            _ => crate::unknown_opcode!("controller", code),
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}
