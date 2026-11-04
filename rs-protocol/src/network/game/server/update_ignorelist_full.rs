#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::jstr_len;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;

#[cfg(since_647)]
pub struct IgnoreEntry {
    pub name: String,
    pub display_name: String,
    pub prev_name: String,
    pub prev_display_name: String,
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
            buf.pjstr(&entry.name, 0);
            buf.pjstr(&entry.display_name, 0);
            buf.pjstr(&entry.prev_name, 0);
            buf.pjstr(&entry.prev_display_name, 0);
        }
    }

    fn sizeof(&self) -> usize {
        1 + self
            .entries
            .iter()
            .map(|e| {
                jstr_len(&e.name)
                    + jstr_len(&e.display_name)
                    + jstr_len(&e.prev_name)
                    + jstr_len(&e.prev_display_name)
            })
            .sum::<usize>()
    }
}
