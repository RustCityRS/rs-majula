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
#[server_prot(IfSetEvents, Buffered, Fixed)]
pub struct IfSetEvents {
    pub com: i32,
    pub slot1: u16,
    pub slot2: u16,
    pub settings: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetEvents {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.slot1);
        buf.p4_alt3(self.settings);
        buf.p4_alt3(self.com);
        buf.ip2_alt1(self.slot2);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
            + size_of_val(&self.slot1)
            + size_of_val(&self.slot2)
            + size_of_val(&self.settings)
    }
}
