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
#[server_prot(IfOpentop, Buffered, Fixed)]
pub struct IfOpentop {
    pub interface: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfOpentop {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.interface);
        buf.p1_alt1(0);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.interface) + 1
    }
}
