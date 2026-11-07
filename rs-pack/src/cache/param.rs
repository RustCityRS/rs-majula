use super::ScriptVarType;
use super::provider::{CacheType, TypeProvider};
use crate::ParamValue;
use rs_io::packet::Packet;
use rs_ver::STRING_TERMINATOR;
use rustc_hash::FxHashMap;

pub type ParamTypeProvider = TypeProvider<ParamType>;

pub struct ParamType {
    pub id: u16,
    pub var_type: ScriptVarType,
    pub default_int: i32,
    pub autodisable: bool,
    pub default_str: Option<Box<str>>,
    debugname: Option<Box<str>>,
}

impl ParamType {
    pub fn decode_params(buf: &mut Packet, params: &mut FxHashMap<i32, ParamValue>) {
        for _ in 0..buf.g1() {
            let (key, value) = Self::decode_param(buf);
            params.insert(key, value);
        }
    }

    pub(crate) fn decode_param(buf: &mut Packet) -> (i32, ParamValue) {
        let is_string = buf.g1() == 1;
        let key = buf.g3();
        let value = if is_string {
            ParamValue::String(buf.gjstr(STRING_TERMINATOR).into_boxed_str())
        } else {
            ParamValue::Int(buf.g4s())
        };
        (key, value)
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

pub struct ParamTypeRaw {
    pub id: u16,
    pub var_type: ScriptVarType,
    pub default_int: i32,
    pub autodisable: bool,
    pub default_str: Option<Box<str>>,
    debugname: Option<Box<str>>,
    pub opcode_order: Vec<u8>,
}

impl CacheType for ParamTypeRaw {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            var_type: ScriptVarType::Int,
            default_int: if cfg!(since_647) { 0 } else { -1 },
            autodisable: true,
            default_str: None,
            debugname: None,
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
                1 => self.var_type = ScriptVarType::try_from(buf.g1()).unwrap(),
                2 => self.default_int = buf.g4s(),
                4 => self.autodisable = false,
                5 => self.default_str = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                250 => self.debugname = Some(buf.gjstr(STRING_TERMINATOR).into_boxed_str()),
                _ => crate::unknown_opcode!("param", code),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}

impl From<ParamTypeRaw> for ParamType {
    fn from(p: ParamTypeRaw) -> Self {
        ParamType {
            id: p.id,
            var_type: p.var_type,
            default_int: p.default_int,
            autodisable: p.autodisable,
            default_str: p.default_str,
            debugname: p.debugname,
        }
    }
}

impl ParamType {
    pub fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
