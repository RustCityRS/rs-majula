use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(ObjAdd, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct ObjAdd {
    pub coord: u8,
    pub id: u16,
    pub count: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for ObjAdd {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.id);
        buf.p2(self.count);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.id) + size_of_val(&self.count)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for ObjAdd {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2_alt1(self.count);
        buf.p1(self.coord);
        buf.ip2_alt1(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.id) + size_of_val(&self.count)
    }
}
