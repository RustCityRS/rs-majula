use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(ObjCount, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct ObjCount {
    pub coord: u8,
    pub id: u16,
    pub old_count: u16,
    pub new_count: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for ObjCount {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.id);
        buf.p2(self.old_count);
        buf.p2(self.new_count);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.id)
            + size_of_val(&self.old_count)
            + size_of_val(&self.new_count)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for ObjCount {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.id);
        buf.p2(self.old_count);
        buf.p2(self.new_count);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.id)
            + size_of_val(&self.old_count)
            + size_of_val(&self.new_count)
    }
}
