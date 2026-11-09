use super::provider::{CacheType, TypeProvider};
use crate::types::ParamValue;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;

pub type ControllerTypeProvider = TypeProvider<ControllerType>;

/// A controller type: the values its configs give its varcons.
pub struct ControllerType {
    pub id: u16,
    pub vars: Box<[(u16, ParamValue)]>,
    debugname: Option<Box<str>>,
}

impl CacheType for ControllerType {
    type Context = ();

    fn new(id: u16) -> Self {
        ControllerType {
            id,
            vars: Box::new([]),
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
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("controller", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
