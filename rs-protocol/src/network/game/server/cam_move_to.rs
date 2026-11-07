#[cfg(before_647)]
use crate::network::game::server::ServerProtMessage;
#[cfg(before_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(before_647)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(before_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(before_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(before_647)]
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(CamMoveTo, Buffered, Fixed)]
pub struct CamMoveTo {
    pub x: u8,
    pub z: u8,
    pub height: u16,
    pub rate: u8,
    pub rate2: u8,
}

#[cfg(before_647)]
impl ServerProtMessage for CamMoveTo {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.x);
        buf.p1(self.z);
        buf.p2(self.height);
        buf.p1(self.rate);
        buf.p1(self.rate2);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x)
            + size_of_val(&self.z)
            + size_of_val(&self.height)
            + size_of_val(&self.rate)
            + size_of_val(&self.rate2)
    }
}
