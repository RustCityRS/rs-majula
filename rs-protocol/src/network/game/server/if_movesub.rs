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
#[server_prot(IfMovesub, Buffered, Fixed)]
pub struct IfMovesub {
    pub com: i32,
    pub com2: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for IfMovesub {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.com2);
        buf.p4_alt2(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.com2)
    }
}
