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

#[cfg(since_647)]
#[server_prot(IfSetVideo, Buffered, Fixed)]
pub struct IfSetVideo {
    pub video: u16,
    pub com: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetVideo {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.video);
        buf.p4(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.video) + size_of_val(&self.com)
    }
}
