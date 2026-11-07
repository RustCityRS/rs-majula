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
#[server_prot(CamRemoveroof, Buffered, Fixed)]
pub struct CamRemoveroof {
    pub coord: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for CamRemoveroof {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.coord);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
    }
}
