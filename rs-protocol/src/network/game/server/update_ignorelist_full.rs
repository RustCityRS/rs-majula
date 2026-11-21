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
pub struct IgnoreEntry {
    pub name: String,
    pub display_name: String,
    pub prev_name: String,
    pub prev_display_name: String,
}

#[cfg(since_647)]
impl IgnoreEntry {
    pub fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.name, STRING_TERMINATOR);
        buf.pjstr(&self.display_name, STRING_TERMINATOR);
        buf.pjstr(&self.prev_name, STRING_TERMINATOR);
        buf.pjstr(&self.prev_display_name, STRING_TERMINATOR);
    }

    pub fn sizeof(&self) -> usize {
        Packet::sizejstr(&self.name)
            + Packet::sizejstr(&self.display_name)
            + Packet::sizejstr(&self.prev_name)
            + Packet::sizejstr(&self.prev_display_name)
    }
}

#[cfg(since_647)]
#[server_prot(UpdateIgnorelistFull, Buffered, VarShort)]
pub struct UpdateIgnorelistFull {
    pub entries: Vec<IgnoreEntry>,
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateIgnorelistFull {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.entries.len() as u8);
        for entry in &self.entries {
            entry.encode(buf);
        }
    }

    fn sizeof(&self) -> usize {
        1 + self.entries.iter().map(IgnoreEntry::sizeof).sum::<usize>()
    }
}
