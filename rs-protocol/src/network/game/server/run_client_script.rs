//! `RunClientScript` — invoke a CS2 client-side script by ID with typed params.
//! Rev-647 opcode 122 (RUNCLIENTSCRIPT), VarShort framing.
//!
//! Wire layout: `[pstr type_string][params in reverse][p4 script_id]`.
//!
//! `type_string` describes the params in argument order — `'i'` for int,
//! `'s'` for string. The params follow last first (i32 big-endian for
//! ints, null-terminated string for strings): the client reads the types
//! from the end of the string back, each value into its type's position
//! (`MapAreaConfig`, RUNCLIENTSCRIPT).

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

pub enum ScriptParam {
    Int(i32),
    Str(String),
}

#[server_prot(RunClientScript, Buffered, VarShort)]
pub struct RunClientScript {
    pub script_id: i32,
    pub params: Vec<ScriptParam>,
}

impl ServerProtMessage for RunClientScript {
    fn encode(&self, buf: &mut Packet) {
        let types: String = self
            .params
            .iter()
            .map(|p| match p {
                ScriptParam::Int(_) => 'i',
                ScriptParam::Str(_) => 's',
            })
            .collect();
        buf.pjstr(&types, 0);
        for param in self.params.iter().rev() {
            match param {
                ScriptParam::Int(v) => buf.p4(*v),
                ScriptParam::Str(s) => buf.pjstr(s, 0),
            }
        }
        buf.p4(self.script_id);
    }

    fn sizeof(&self) -> usize {
        let mut n = self.params.len() + 1 + 4;
        for param in &self.params {
            n += match param {
                ScriptParam::Int(_) => 4,
                ScriptParam::Str(s) => s.len() + 1,
            };
        }
        n
    }
}
