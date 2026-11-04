use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use rs_io::packet::Packet;
use rustc_hash::FxHashMap;

pub type ParamTypeProvider = TypeProvider<ParamType>;

pub struct ParamType {
    pub id: u16,
    pub var_type: ScriptVarType,
    pub default_int: i32,
    pub default_str: Option<Box<str>>,
    pub autodisable: bool,
    pub(crate) debugname: Option<Box<str>>,
}

impl ParamType {
    pub fn decode_params(buf: &mut Packet, params: &mut FxHashMap<i32, ParamValue>) {
        let count: u8 = buf.g1();
        for _ in 0..count {
            let key = buf.g3();
            if buf.g1() == 1 {
                params.insert(key, ParamValue::String(buf.gjstr(10).into_boxed_str()));
            } else {
                params.insert(key, ParamValue::Int(buf.g4s()));
            }
        }
    }

    pub fn get_param_or_default(
        &self,
        params: &'static FxHashMap<i32, ParamValue>,
    ) -> Option<&'static ParamValue> {
        params.get(&(self.id as i32))
    }

    pub fn default_param(&self) -> ParamValue {
        if self.var_type == ScriptVarType::String {
            ParamValue::String(self.default_str.clone().unwrap_or_else(|| "null".into()))
        } else {
            ParamValue::Int(self.default_int)
        }
    }
}

impl CacheType for ParamType {
    type Context = ();

    fn new(id: u16) -> Self {
        ParamType {
            id,
            var_type: ScriptVarType::Int,
            default_int: -1,
            default_str: None,
            autodisable: true,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => self.var_type = ScriptVarType::try_from(buf.g1()).unwrap(),
                2 => self.default_int = buf.g4s(),
                4 => self.autodisable = false,
                5 => self.default_str = Some(buf.gjstr(10).into_boxed_str()),
                250 => self.debugname = Some(buf.gjstr(10).into_boxed_str()),
                _ => crate::unknown_opcode!("param", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
