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
#[server_prot(UpdateFriendchatChannelSingleuser, Buffered, VarByte)]
pub struct UpdateFriendchatChannelSingleuser {
    pub name: String,
    pub display_name: Option<String>,
    pub world: u16,
    pub rank: i8,
    pub title: Option<String>,
}

#[cfg(since_647)]
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
        let name = jstr_len(&self.name) + 1 + self.display_name.as_ref().map_or(0, |d| jstr_len(d));
        let tail = size_of_val(&self.world)
            + size_of_val(&self.rank)
            + if self.rank != -128 {
                self.title.as_ref().map_or(1, |t| jstr_len(t))
            } else {
                0
            };
        name + tail
    }
}
