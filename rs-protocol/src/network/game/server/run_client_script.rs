#[cfg(since_530)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_530)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_530)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_530)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_530)]
use rs_protocol_macros::server_prot;
use rs_ver::STRING_TERMINATOR;

#[cfg(since_530)]
pub enum ScriptParam {
    Int(i32),
    Str(String),
}

#[cfg(since_530)]
#[server_prot(RunClientScript, Buffered, VarShort)]
pub struct RunClientScript {
    pub script_id: i32,
    pub params: Vec<ScriptParam>,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(since_530)]
impl ServerProtMessage for RunClientScript {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(all(since_530, before_647))]
        buf.p2(self.verify);
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
        #[cfg(all(since_530, before_647))]
        {
            n += size_of_val(&self.verify);
        }
        n
    }
}
