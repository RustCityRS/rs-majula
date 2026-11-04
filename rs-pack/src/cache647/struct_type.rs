use std::collections::HashMap;

use rs_io::Packet;

use super::{CacheType, ParamValue};

/// Archive 2, group 26 (CONFIG_STRUCT).
///
/// | op  | shape                                                        |
/// |-----|--------------------------------------------------------------|
/// | 249 | g1 count, count x (g1 is_string, g3 key, g4 int / gjstr str) |
pub struct StructType {
    pub id: u16,
    /// Params in original binary insertion order (not sorted by key).
    pub params: Vec<(i32, ParamValue)>,
}

impl CacheType for StructType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            params: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        loop {
            let opcode = buf.g1();
            if opcode == 0 {
                break;
            }
            match opcode {
                249 => {
                    let count = buf.g1() as usize;
                    self.params.reserve(count);
                    for _ in 0..count {
                        let is_string = buf.g1() == 1;
                        let key = buf.g3();
                        let value = if is_string {
                            ParamValue::String(buf.gjstr(0).into_boxed_str())
                        } else {
                            ParamValue::Int(buf.g4s())
                        };
                        self.params.push((key, value));
                    }
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

impl StructType {
    pub fn get_string(&self, key: i32) -> Option<&str> {
        self.params.iter().find_map(|(k, v)| {
            if *k == key {
                if let ParamValue::String(s) = v {
                    Some(s.as_ref())
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn get_int(&self, key: i32) -> Option<i32> {
        self.params.iter().find_map(|(k, v)| {
            if *k == key {
                if let ParamValue::Int(i) = v {
                    Some(*i)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

/// Decode opcode 249 param bag. Shared by NPC/OBJ/LOC/Struct types.
pub fn decode_params(buf: &mut Packet, params: &mut HashMap<i32, ParamValue>) {
    let count = buf.g1() as usize;
    params.reserve(count);
    for _ in 0..count {
        let is_string = buf.g1() == 1;
        let key = buf.g3();
        let value = if is_string {
            ParamValue::String(buf.gjstr(0).into_boxed_str())
        } else {
            ParamValue::Int(buf.g4s())
        };
        params.insert(key, value);
    }
}
