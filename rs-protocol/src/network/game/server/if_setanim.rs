use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetAnim, Buffered, Fixed)]
pub struct IfSetAnim {
    pub com: u16,
    pub seq: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.seq);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.seq)
    }
}

#[cfg(since_647)]
#[server_prot(IfSetAnim, Buffered, Fixed)]
pub struct IfSetAnim {
    pub com: i32,
    pub seq: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.ip4(self.com);
        buf.ip2(self.seq);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.seq)
    }
}
