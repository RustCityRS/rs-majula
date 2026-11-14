#[cfg(since_530)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_530)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_530)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_530)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_530)]
use rs_protocol_macros::server_prot;

#[cfg(since_530)]
#[server_prot(CamForceangle, Buffered, Fixed)]
pub struct CamForceangle {
    pub yaw: u16,
    pub pitch: u16,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for CamForceangle {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.yaw);
        buf.p2(self.verify);
        buf.p2(self.pitch);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.yaw) + size_of_val(&self.pitch) + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for CamForceangle {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        buf.p2_alt1(self.yaw);
        buf.p2_alt2(self.pitch);
        #[cfg(since_742)]
        buf.p2(self.yaw);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.yaw) + size_of_val(&self.pitch)
    }
}
