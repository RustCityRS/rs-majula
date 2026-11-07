use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(UpdateStat, Buffered, Fixed)]
pub struct UpdateStat {
    pub stat: u8,
    pub exp: i32,
    pub lvl: u8,
}

#[cfg(before_647)]
impl ServerProtMessage for UpdateStat {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.stat);
        buf.p4(self.exp);
        buf.p1(self.lvl);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.stat) + size_of_val(&self.exp) + size_of_val(&self.lvl)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateStat {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.exp);
        buf.p1(self.stat);
        buf.p1_alt2(self.lvl);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.stat) + size_of_val(&self.exp) + size_of_val(&self.lvl)
    }
}
