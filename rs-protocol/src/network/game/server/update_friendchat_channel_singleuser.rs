use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(UpdateFriendchatChannelSingleuser, Buffered, VarByte)]
pub struct UpdateFriendchatChannelSingleuser {
    pub name: String,
    pub display_name: Option<String>,
    pub world: u16,
    pub rank: i8,
    pub title: Option<String>,
}

impl ServerProtMessage for UpdateFriendchatChannelSingleuser {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.name, 0);
        if let Some(ref display) = self.display_name {
            buf.p1(1);
            buf.pjstr(display, 0);
        } else {
            buf.p1(0);
        }
        buf.p2(self.world);
        buf.p1(self.rank as u8);
        if self.rank != -128 {
            buf.pjstr(self.title.as_deref().unwrap_or(""), 0);
        }
    }

    fn sizeof(&self) -> usize {
        let name = self.name.len() + 1 + 1 + self.display_name.as_ref().map_or(0, |d| d.len() + 1);
        let tail = 2
            + 1
            + if self.rank != -128 {
                self.title.as_ref().map_or(1, |t| t.len() + 1)
            } else {
                0
            };
        name + tail
    }
}
