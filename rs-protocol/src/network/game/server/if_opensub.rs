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
#[server_prot(IfOpensub, Buffered, Fixed)]
pub struct IfOpensub {
    pub interface: u16,
    pub com: i32,
    pub walkable: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for IfOpensub {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt3(self.interface);
        buf.p4_alt3(self.com);
        buf.p1_alt1(self.walkable);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.interface) + size_of_val(&self.com) + size_of_val(&self.walkable)
    }
}
