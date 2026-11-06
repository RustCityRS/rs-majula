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
#[server_prot(IfSetRetex, Buffered, Fixed)]
pub struct IfSetRetex {
    pub com: i32,
    pub index: u8,
    pub src: u16,
    pub dst: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetRetex {
    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.com);
        buf.p1_alt3(self.index);
        buf.p2_alt2(self.src);
        buf.p2_alt2(self.dst);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
            + size_of_val(&self.index)
            + size_of_val(&self.src)
            + size_of_val(&self.dst)
    }
}
