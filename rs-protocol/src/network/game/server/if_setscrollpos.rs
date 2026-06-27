#[cfg(since_245_2)]
use crate::network::game::server::ServerProtMessage;
#[cfg(since_245_2)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_245_2)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_245_2)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_245_2)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_245_2)]
use rs_protocol_macros::server_prot;

#[cfg(since_245_2)]
#[server_prot(IfSetScrollPos, Buffered, Fixed)]
pub struct IfSetScrollPos {
    pub com: u16,
    pub y: u16,
}

#[cfg(since_245_2)]
impl ServerProtMessage for IfSetScrollPos {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.y);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.y)
    }
}
