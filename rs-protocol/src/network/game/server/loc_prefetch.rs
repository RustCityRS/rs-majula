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
#[server_prot(LocPrefetch, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct LocPrefetch {
    pub id: u16,
    pub shape: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for LocPrefetch {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.id);
        buf.p1(self.shape);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.shape)
    }
}
