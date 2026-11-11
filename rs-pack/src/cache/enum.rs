use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use rs_io::Packet;
use rs_ver::STRING_TERMINATOR;
use rustc_hash::FxHashMap;

pub type EnumTypeProvider = TypeProvider<EnumType>;

pub struct EnumType {
    pub id: u16,
    pub inputtype: ScriptVarType,
    pub outputtype: ScriptVarType,
    pub default_str: Option<Box<str>>,
    pub default_int: i32,
    pub values: FxHashMap<i32, ParamValue>,
    pub debugname: Box<str>,
}

pub struct EnumTypeRaw {
    pub id: u16,
    pub inputtype: ScriptVarType,
    pub outputtype: ScriptVarType,
    pub default_str: Option<Box<str>>,
    pub default_int: i32,
    pub values: Vec<(i32, ParamValue)>,
    debugname: Box<str>,
    pub opcode_order: Vec<u8>,
}

impl CacheType for EnumTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            inputtype: ScriptVarType::Int,
            outputtype: ScriptVarType::Int,
            default_str: None,
            default_int: 0,
            values: Vec::new(),
            debugname: Box::default(),
            opcode_order: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code = buf.g1();
            if code == 0 {
                break;
            }
            self.opcode_order.push(code);
            match code {
                1 => self.inputtype = ScriptVarType::try_from(buf.g1()).unwrap(),
                2 => self.outputtype = ScriptVarType::try_from(buf.g1()).unwrap(),
                3 => self.default_str = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                4 => self.default_int = buf.g4s(),
                5 | 6 => {
                    let count = buf.g2() as usize;
                    self.values.reserve(count);
                    for _ in 0..count {
                        let key = buf.g4s();
                        let value = if code == 5 {
                            ParamValue::String(buf.gjstr(STRING_TERMINATOR).into_boxed_str())
                        } else {
                            ParamValue::Int(buf.g4s())
                        };
                        self.values.push((key, value));
                    }
                }
                250 => self.debugname = buf.gjstr(STRING_TERMINATOR).into_boxed_str(),
                _ => crate::unknown_opcode!("enum", code),
            }
        }
    }

    fn debugname(&self) -> &str {
        &self.debugname
    }
}

impl From<EnumTypeRaw> for EnumType {
    fn from(e: EnumTypeRaw) -> Self {
        EnumType {
            id: e.id,
            inputtype: e.inputtype,
            outputtype: e.outputtype,
            default_str: e.default_str,
            default_int: e.default_int,
            values: e.values.into_iter().collect(),
            debugname: e.debugname,
        }
    }
}
