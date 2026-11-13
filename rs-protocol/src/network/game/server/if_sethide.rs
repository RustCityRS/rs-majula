use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetHide, Buffered, Fixed)]
pub struct IfSetHide {
    pub com: u16,
    pub hide: bool,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetHide {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p1(self.hide as u8);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.hide)
    }
}

#[cfg(since_647)]
#[server_prot(IfSetHide, Buffered, Fixed)]
pub struct IfSetHide {
    pub com: i32,
    pub hidden: bool,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetHide {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt3(self.com);
        buf.p1_alt2(self.hidden as u8);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.hidden)
    }
}
