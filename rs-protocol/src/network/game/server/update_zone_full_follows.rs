use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(UpdateZoneFullFollows, Immediate, Fixed)]
pub struct UpdateZoneFullFollows {
    pub x: u8,
    pub z: u8,
}

#[cfg(before_647)]
impl ServerProtMessage for UpdateZoneFullFollows {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.x);
        buf.p1(self.z);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z)
    }
}

#[cfg(since_647)]
#[server_prot(UpdateZoneFullFollows, Immediate, Fixed)]
pub struct UpdateZoneFullFollows {
    pub x: u8,
    pub z: u8,
    pub y: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateZoneFullFollows {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt1(self.y);
        buf.p1(self.x);
        buf.p1_alt2(self.z);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z) + size_of_val(&self.y)
    }
}
