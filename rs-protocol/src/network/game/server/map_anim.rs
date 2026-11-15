use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[server_prot(MapAnim, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct MapAnim {
    pub coord: u8,
    pub spotanim: TypeId,
    pub height: u8,
    pub delay: u16,
    #[cfg(since_742)]
    pub trailing: i16,
}

#[cfg(before_647)]
impl ServerProtMessage for MapAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.spotanim.g2());
        buf.p1(self.height);
        buf.p2(self.delay);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of::<u16>()
            + size_of_val(&self.height)
            + size_of_val(&self.delay)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for MapAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.spotanim.g2());
        #[cfg(before_742)]
        buf.p1(self.height);
        #[cfg(since_742)]
        buf.p2(self.height as u16);
        buf.p2(self.delay);
        buf.p1(0);
        #[cfg(since_742)]
        buf.p2(self.trailing as u16);
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let (height, trailing) = (size_of_val(&self.height), 0);
        #[cfg(since_742)]
        let (height, trailing) = (size_of::<u16>(), size_of_val(&self.trailing));
        size_of_val(&self.coord)
            + size_of::<u16>()
            + height
            + size_of_val(&self.delay)
            + size_of::<u8>()
            + trailing
    }
}
