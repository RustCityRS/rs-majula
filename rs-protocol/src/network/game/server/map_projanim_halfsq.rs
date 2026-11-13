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
#[server_prot(MapProjAnimHalfsq, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct MapProjAnimHalfsq {
    pub coord: u8,
    pub ground: bool,
    pub wearpos: Option<u8>,
    pub dx: i8,
    pub dz: i8,
    pub source: i16,
    pub target: i16,
    pub spotanim: u16,
    pub src_height: u8,
    pub dst_height: u8,
    pub start_delay: u16,
    pub end_delay: u16,
    pub peak: u8,
    pub arc: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for MapProjAnimHalfsq {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p1(self.wearpos.map_or(0, |wearpos| wearpos << 2 | 0x2) | self.ground as u8);
        buf.p1(self.dx as u8);
        buf.p1(self.dz as u8);
        buf.p2(self.source as u16);
        buf.p2(self.target as u16);
        buf.p2(self.spotanim);
        buf.p1(self.src_height);
        buf.p1(self.dst_height);
        buf.p2(self.start_delay);
        buf.p2(self.end_delay);
        buf.p1(self.peak);
        buf.p2(self.arc);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + 1 // ground, wearpos
            + size_of_val(&self.dx)
            + size_of_val(&self.dz)
            + size_of_val(&self.source)
            + size_of_val(&self.target)
            + size_of_val(&self.spotanim)
            + size_of_val(&self.src_height)
            + size_of_val(&self.dst_height)
            + size_of_val(&self.start_delay)
            + size_of_val(&self.end_delay)
            + size_of_val(&self.peak)
            + size_of_val(&self.arc)
    }
}
