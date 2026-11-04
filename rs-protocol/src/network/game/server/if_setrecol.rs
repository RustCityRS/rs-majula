#[cfg(any(before_245_2, since_647))]
use crate::network::game::server::ServerProtMessage;
#[cfg(any(before_245_2, since_647))]
use crate::network::game::server_prot::ServerProt;
#[cfg(any(before_245_2, since_647))]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(any(before_245_2, since_647))]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(any(before_245_2, since_647))]
use rs_io::{Packet, PacketFrame};
#[cfg(any(before_245_2, since_647))]
use rs_protocol_macros::server_prot;

#[cfg(before_245_2)]
#[server_prot(IfSetRecol, Buffered, Fixed)]
pub struct IfSetRecol {
    pub com: u16,
    pub src: u16,
    pub dst: u16,
}

#[cfg(before_245_2)]
impl ServerProtMessage for IfSetRecol {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.src);
        buf.p2(self.dst);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.src) + size_of_val(&self.dst)
    }
}

#[cfg(since_647)]
#[server_prot(IfSetRecol, Buffered, Fixed)]
pub struct IfSetRecol {
    pub com: i32,
    pub src: u16,
    pub dst: u16,
    pub slot: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetRecol {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.slot);
        buf.p2(self.src);
        buf.p2(self.dst);
        buf.p4(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
            + size_of_val(&self.src)
            + size_of_val(&self.dst)
            + size_of_val(&self.slot)
    }
}
