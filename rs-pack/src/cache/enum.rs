use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use rs_io::Packet;
use rustc_hash::FxHashMap;

pub type EnumTypeProvider = TypeProvider<EnumType>;

pub struct EnumType {
    pub id: u16,
    pub inputtype: ScriptVarType,
    pub outputtype: ScriptVarType,
    pub default_int: i32,
    pub default_str: Option<Box<str>>,
    pub values: FxHashMap<i32, ParamValue>,
    pub(crate) debugname: Option<Box<str>>,
}

impl CacheType for EnumType {
    type Context = ();

    fn new(id: u16) -> Self {
        EnumType {
            id,
            inputtype: ScriptVarType::Int,
            outputtype: ScriptVarType::Int,
            default_int: 0,
            default_str: None,
            values: FxHashMap::default(),
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.inputtype = ScriptVarType::try_from(buf.g1()).unwrap(),
                2 => self.outputtype = ScriptVarType::try_from(buf.g1()).unwrap(),
                3 => self.default_str = Some(buf.gjstr(10).into_boxed_str()),
                4 => self.default_int = buf.g4s(),
                5 => {
                    let count = buf.g2() as usize;
                    for _ in 0..count {
                        self.values.insert(
                            buf.g4s(),
                            ParamValue::String(buf.gjstr(10).into_boxed_str()),
                        );
                    }
                }
                6 => {
                    let count = buf.g2() as usize;
                    for _ in 0..count {
                        self.values.insert(buf.g4s(), ParamValue::Int(buf.g4s()));
                    }
                }
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => crate::unknown_opcode!("enum", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
