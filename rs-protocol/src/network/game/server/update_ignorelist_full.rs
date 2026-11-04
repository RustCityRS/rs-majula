use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

pub struct IgnoreEntry {
    pub name: String,
    pub display_name: String,
    pub prev_name: String,
    pub prev_display_name: String,
}

#[server_prot(UpdateIgnorelistFull, Buffered, VarShort)]
pub struct UpdateIgnorelistFull {
    pub entries: Vec<IgnoreEntry>,
}

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
                e.name.len()
                    + 1
                    + e.display_name.len()
                    + 1
                    + e.prev_name.len()
                    + 1
                    + e.prev_display_name.len()
                    + 1
            })
            .sum::<usize>()
    }
}
