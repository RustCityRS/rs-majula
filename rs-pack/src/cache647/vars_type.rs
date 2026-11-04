use rs_io::Packet;

use super::CacheType;
use crate::types647::ScriptVarType;

/// The server-only shared (world) var type, `vars.dat` (`pack647/vars.rs`):
/// the same opcodes as [`super::VarnType`].
///
/// | op  | shape                                |
/// |-----|--------------------------------------|
/// | 1   | g1 (var_type: ScriptVarType char)     |
/// | 250 | gjstr (debugname)                    |
pub struct VarsType {
    pub id: u16,
    pub var_type: ScriptVarType,
    debugname: Option<Box<str>>,
}

impl CacheType for VarsType {
    type Context = ();

    fn new(id: u16) -> Self {
        Self {
            id,
            var_type: ScriptVarType::Int,
            debugname: None,
        }
    }

    fn decode(&mut self, buf: &mut Packet) {
        while buf.remaining() > 0 {
            let code: u8 = buf.g1();
            match code {
                0 => break,
                1 => {
                    self.var_type = ScriptVarType::try_from(buf.g1()).unwrap_or(ScriptVarType::Int);
                }
                250 => self.debugname = Some(buf.gjstr(0).into_boxed_str()),
                _ => panic!("Unrecognized vars config code: {code}"),
            }
        }
    }

    fn debugname(&self) -> Option<&str> {
        self.debugname.as_deref()
    }
}
