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
#[server_prot(SetMapFlag, Buffered, Fixed)]
pub struct SetMapFlag {
    pub x: u8,
    pub z: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for SetMapFlag {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt3(self.x);
        buf.p1_alt3(self.z);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z)
    }
}
