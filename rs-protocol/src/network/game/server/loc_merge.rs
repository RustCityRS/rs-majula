use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[server_prot(LocMerge, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct LocMerge {
    pub coord: u8,
    pub shape_angle: u8,
    pub id: TypeId,
    pub start: u16,
    pub end: u16,
    pub pid: u16,
    pub east: i8,
    pub south: i8,
    pub west: i8,
    pub north: i8,
}

#[cfg(before_530)]
impl ServerProtMessage for LocMerge {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p1(self.shape_angle);
        buf.p2(self.id.g2());
        buf.p2(self.start);
        buf.p2(self.end);
        buf.p2(self.pid);
        buf.p1(self.east as u8);
        buf.p1(self.south as u8);
        buf.p1(self.west as u8);
        buf.p1(self.north as u8);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.shape_angle)
            + size_of::<u16>()
            + size_of_val(&self.start)
            + size_of_val(&self.end)
            + size_of_val(&self.pid)
            + size_of_val(&self.south)
            + size_of_val(&self.east)
            + size_of_val(&self.north)
            + size_of_val(&self.west)
    }
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for LocMerge {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.shape_angle);
        buf.p1(self.coord);
        buf.p1_alt1(self.east as u8);
        buf.p1_alt1(self.south as u8);
        buf.p1_alt3(self.north as u8);
        buf.p2_alt2(self.id.g2());
        buf.p2_alt1(self.end);
        buf.p1(self.west as u8);
        buf.p2(self.start);
        buf.p2_alt3(!self.pid);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.shape_angle)
            + size_of_val(&self.id)
            + size_of_val(&self.start)
            + size_of_val(&self.end)
            + size_of_val(&self.pid)
            + size_of_val(&self.south)
            + size_of_val(&self.east)
            + size_of_val(&self.north)
            + size_of_val(&self.west)
    }
}
