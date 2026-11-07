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
#[server_prot(IfSetGraphic, Buffered, Fixed)]
pub struct IfSetGraphic {
    pub com: i32,
    pub graphic: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetGraphic {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt1(self.com);
        buf.p2_alt2(self.graphic);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.graphic)
    }
}
