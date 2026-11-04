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
#[server_prot(IfSetTargetParam, Buffered, Fixed)]
pub struct IfSetTargetParam {
    pub com: i32,
    pub slot1: u16,
    pub slot2: u16,
    pub inv: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetTargetParam {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.slot1);
        buf.ip2_alt1(self.inv);
        buf.p2_alt1(self.slot2);
        buf.p4_alt3(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
            + size_of_val(&self.slot1)
            + size_of_val(&self.slot2)
            + size_of_val(&self.inv)
    }
}
