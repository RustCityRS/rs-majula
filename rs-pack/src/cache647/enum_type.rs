use std::collections::HashMap;

use rs_io::Packet;

use super::CacheType;

/// Enum value — either an integer or a string.
#[derive(Debug, Clone)]
pub enum EnumValue {
    Int(i32),
    String(Box<str>),
}

/// Archive 17 (CONFIG_ENUM), strided u32 ids.
///
/// | op    | shape                                                          |
/// |-------|----------------------------------------------------------------|
/// | 1     | g1 (key_type — CP1252 char)                                    |
/// | 2     | g1 (value_type — CP1252 char)                                  |
/// | 3     | gjstr (default string)                                         |
/// | 4     | g4 (default int)                                               |
/// | 5 / 6 | g2 size, size x (g4 key, gjstr value [op 5] / g4 value [op 6]) |
pub struct EnumType {
    pub id: u16,
    pub key_type: u8,
    pub value_type: u8,
    pub default_string: Box<str>,
    pub default_int: i32,
    pub entries: HashMap<i32, EnumValue>,
    /// Entries in decode order, including duplicates. Used by the emitter
    /// to produce byte-perfect cache binary. The `entries` HashMap is kept
    /// for runtime lookups (last-wins for duplicates, matching the client).
    pub ordered_entries: Vec<(i32, EnumValue)>,
    /// Opcodes in the order they were decoded. Used by the emitter to
    /// preserve the original opcode ordering (e.g. entries before defaults).
    pub opcode_order: Vec<u8>,
}

impl CacheType for EnumType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            key_type: 0,
            value_type: 0,
            default_string: String::new().into_boxed_str(),
            default_int: 0,
            entries: HashMap::new(),
            ordered_entries: Vec::new(),
            opcode_order: Vec::new(),
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        loop {
            let opcode = buf.g1();
            if opcode == 0 {
                break;
            }
            self.opcode_order.push(opcode);
            match opcode {
                1 => self.key_type = buf.g1(),
                2 => self.value_type = buf.g1(),
                3 => self.default_string = buf.gjstr(0).into_boxed_str(),
                4 => self.default_int = buf.g4s(),
                5 => {
                    let count = buf.g2() as usize;
                    self.entries.reserve(count);
                    self.ordered_entries.reserve(count);
                    for _ in 0..count {
                        let key = buf.g4s();
                        let value = EnumValue::String(buf.gjstr(0).into_boxed_str());
                        self.entries.insert(key, value.clone());
                        self.ordered_entries.push((key, value));
                    }
                }
                6 => {
                    let count = buf.g2() as usize;
                    self.entries.reserve(count);
                    self.ordered_entries.reserve(count);
                    for _ in 0..count {
                        let key = buf.g4s();
                        let value = EnumValue::Int(buf.g4s());
                        self.entries.insert(key, value.clone());
                        self.ordered_entries.push((key, value));
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

impl EnumType {
    /// Get a string value by key.
    pub fn get_string(&self, key: i32) -> Option<&str> {
        match self.entries.get(&key) {
            Some(EnumValue::String(s)) => Some(s),
            _ => None,
        }
    }

    /// Get an integer value by key.
    pub fn get_int(&self, key: i32) -> Option<i32> {
        match self.entries.get(&key) {
            Some(EnumValue::Int(i)) => Some(*i),
            _ => None,
        }
    }
}
