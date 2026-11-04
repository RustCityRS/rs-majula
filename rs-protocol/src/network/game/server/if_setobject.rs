use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetObject, Buffered, Fixed)]
pub struct IfSetObject {
    pub com: u16,
    pub obj: u16,
    pub scale: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetObject {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.obj);
        buf.p2(self.scale);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.obj) + size_of_val(&self.scale)
    }
}

#[cfg(since_647)]
#[server_prot(IfSetObject, Buffered, Fixed)]
pub struct IfSetObject {
    pub com: i32,
    pub obj: u16,
    pub scale: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetObject {
    fn encode(&self, buf: &mut Packet) {
        buf.ip4(self.com);
        buf.p4_alt3(self.scale);
        buf.p2(self.obj);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.obj) + size_of_val(&self.scale)
    }
}
