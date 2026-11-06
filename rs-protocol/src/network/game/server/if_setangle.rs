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
#[server_prot(IfSetAngle, Buffered, Fixed)]
pub struct IfSetAngle {
    pub com: i32,
    pub x: u16,
    pub y: u16,
    pub zoom: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetAngle {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.x);
        buf.p4_alt3(self.com);
        buf.p2_alt2(self.y);
        buf.p2_alt2(self.zoom);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
            + size_of_val(&self.x)
            + size_of_val(&self.y)
            + size_of_val(&self.zoom)
    }
}
