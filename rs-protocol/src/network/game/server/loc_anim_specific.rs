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
#[server_prot(LocAnimSpecific, Immediate, Fixed)]
pub struct LocAnimSpecific {
    pub seq: u16,
    pub shape_angle: u8,
    pub coord: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for LocAnimSpecific {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.seq);
        buf.p1_alt2(self.shape_angle);
        buf.p4(self.coord);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.seq) + size_of_val(&self.shape_angle) + size_of_val(&self.coord)
    }
}
