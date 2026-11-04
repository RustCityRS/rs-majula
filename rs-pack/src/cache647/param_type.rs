use std::collections::HashMap;

use rs_io::Packet;

use super::CacheType;
use super::provider::ParamValue;

/// Archive 2, group 11 (CONFIG_PARAM).
///
/// | op | shape                                                  |
/// |----|--------------------------------------------------------|
/// | 1  | g1 (type char: 'i'/'1' = integer, 's'/'a' = string)    |
/// | 2  | g4 (default integer)                                   |
/// | 4  | flag (auto_dispose = false; no payload)                |
/// | 5  | gjstr (default string)                                 |
pub struct ParamType {
    pub id: u16,
    /// Raw type char byte (e.g. 'i', '1', 's', 'a').
    pub type_char: Option<u8>,
    pub default_int: Option<i32>,
    pub default_string: Option<Box<str>>,
    pub auto_disable: bool,
    /// Opcodes in decode order, for byte-perfect re-emission.
    pub opcode_order: Vec<u8>,
}

impl CacheType for ParamType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            type_char: None,
            default_int: None,
            default_string: None,
            auto_disable: true,
            opcode_order: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        loop {
            let opcode = buf.g1();
            if opcode == 0 {
                break;
            }
            match opcode {
                1 => {
                    self.opcode_order.push(opcode);
                    self.type_char = Some(buf.g1());
                }
                2 => {
                    self.opcode_order.push(opcode);
                    self.default_int = Some(buf.g4s());
                }
                4 => {
                    self.opcode_order.push(opcode);
                    self.auto_disable = false;
                }
                5 => {
                    self.opcode_order.push(opcode);
                    self.default_string = Some(buf.gjstr(0).into_boxed_str());
                }
                250 => {
                    let _debugname = buf.gjstr(0);
                }
                _ => {
                    buf.pos = buf.data.len();
                    return;
                }
            }
        }
    }
}

impl ParamType {
    /// Check if this param is a string type.
    pub fn is_string(&self) -> bool {
        self.type_char
            .is_some_and(|c| matches!(c as char, 's' | 'a'))
    }

    /// Check if this param is an integer type.
    pub fn is_integer(&self) -> bool {
        !self.is_string()
    }

    /// Look up this param's value in a param map, returning the value if found,
    /// or `None` if not present (caller should fall back to `default_param()`).
    pub fn get_param_or_default<'a>(
        &self,
        params: &'a HashMap<i32, ParamValue>,
    ) -> Option<&'a ParamValue> {
        params.get(&(self.id as i32))
    }

    /// Return the default `ParamValue` for this param type.
    pub fn default_param(&self) -> ParamValue {
        if self.is_string() {
            ParamValue::String(self.default_string.as_deref().unwrap_or("null").into())
        } else {
            ParamValue::Int(self.default_int.unwrap_or(0))
        }
    }
}
