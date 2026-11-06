use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(ObjDel, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct ObjDel {
    pub coord: u8,
    pub id: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for ObjDel {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.id)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for ObjDel {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.coord);
        buf.p2_alt3(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.id)
    }
}
