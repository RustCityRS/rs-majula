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
#[server_prot(CamForceangle, Buffered, Fixed)]
pub struct CamForceangle {
    pub yaw: u16,
    pub pitch: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for CamForceangle {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.yaw);
        buf.p2_alt1(self.pitch);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.yaw) + size_of_val(&self.pitch)
    }
}
