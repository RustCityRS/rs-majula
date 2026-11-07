use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(MapProjAnim, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct MapProjAnim {
    pub coord: u8,
    pub dx: i8,
    pub dz: i8,
    pub target: i16,
    pub spotanim: u16,
    pub src_height: u8,
    pub dst_height: u8,
    pub start_delay: u16,
    pub end_delay: u16,
    pub peak: u8,
    #[cfg(before_647)]
    pub arc: u8,
    #[cfg(since_647)]
    pub arc: u16,
}

impl ServerProtMessage for MapProjAnim {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_647)]
        buf.p1(self.coord);
        #[cfg(since_647)]
        buf.p1(((self.coord >> 4 & 7) << 3) | (self.coord & 7));
        buf.p1(self.dx as u8);
        buf.p1(self.dz as u8);
        buf.p2(self.target as u16);
        buf.p2(self.spotanim);
        buf.p1(self.src_height);
        buf.p1(self.dst_height);
        buf.p2(self.start_delay);
        buf.p2(self.end_delay);
        buf.p1(self.peak);
        #[cfg(before_647)]
        buf.p1(self.arc);
        #[cfg(since_647)]
        buf.p2(self.arc);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.dx)
            + size_of_val(&self.dz)
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
