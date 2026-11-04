use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(LocAnim, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct LocAnim {
    pub coord: u8,
    pub shape_angle: u8,
    pub seq: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for LocAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p1(self.shape_angle);
        buf.p2(self.seq);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + size_of_val(&self.seq)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for LocAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt1(self.coord);
        buf.p1(self.shape_angle);
        buf.ip2(self.seq);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + size_of_val(&self.seq)
    }
}
