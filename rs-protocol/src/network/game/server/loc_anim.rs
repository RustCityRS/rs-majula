use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[server_prot(LocAnim, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct LocAnim {
    pub coord: u8,
    pub shape_angle: u8,
    pub seq: TypeId,
}

#[cfg(before_647)]
impl ServerProtMessage for LocAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p1(self.shape_angle);
        buf.p2(self.seq.g2());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + 2
    }
}

#[cfg(all(since_647, before_742))]
impl ServerProtMessage for LocAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.coord);
        buf.p1(self.shape_angle);
        buf.p2_alt1(self.seq.g2());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + 2
    }
}

#[cfg(since_742)]
impl ServerProtMessage for LocAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.seq.int());
        buf.p1_alt3(self.shape_angle);
        buf.p1(self.coord);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + size_of::<i32>()
    }
}
