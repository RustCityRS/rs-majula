use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(MapAnim, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct MapAnim {
    pub coord: u8,
    pub spotanim: u16,
    pub height: u8,
    pub delay: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for MapAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.spotanim);
        buf.p1(self.height);
        buf.p2(self.delay);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.spotanim)
            + size_of_val(&self.height)
            + size_of_val(&self.delay)
    }
}
