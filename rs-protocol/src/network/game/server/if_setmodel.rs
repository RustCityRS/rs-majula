use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetModel, Buffered, Fixed)]
pub struct IfSetModel {
    pub com: u16,
    pub model: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetModel {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.model);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.model)
    }
}

#[cfg(since_647)]
#[server_prot(IfSetModel, Buffered, Fixed)]
pub struct IfSetModel {
    pub com: i32,
    pub model: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetModel {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2_alt1(self.model);
        buf.p4(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.model)
    }
}
