#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;
use rs_ver::STRING_TERMINATOR;

#[cfg(since_647)]
pub enum ScriptParam {
    Int(i32),
    Str(String),
}

#[cfg(since_647)]
#[server_prot(RunClientScript, Buffered, VarShort)]
pub struct RunClientScript {
    pub script_id: i32,
    pub params: Vec<ScriptParam>,
}

#[cfg(since_647)]
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
        buf.pjstr(&types, STRING_TERMINATOR);
        for param in self.params.iter().rev() {
            match param {
                ScriptParam::Int(v) => buf.p4(*v),
                ScriptParam::Str(s) => buf.pjstr(s, STRING_TERMINATOR),
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
