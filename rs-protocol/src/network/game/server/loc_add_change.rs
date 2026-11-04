use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(LocAddChange, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct LocAddChange {
    pub coord: u8,
    pub shape_angle: u8,
    pub id: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for LocAddChange {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p1(self.shape_angle);
        buf.p2(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + size_of_val(&self.id)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for LocAddChange {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.coord);
        buf.p1_alt1(self.shape_angle);
        buf.p2_alt1(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + size_of_val(&self.id)
    }
}
