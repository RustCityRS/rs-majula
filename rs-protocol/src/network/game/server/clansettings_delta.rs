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
#[server_prot(ClansettingsDelta, Buffered, VarShort)]
pub struct ClansettingsDelta {
    pub affined: bool,
    pub data: Vec<u8>,
}

#[cfg(since_647)]
impl ServerProtMessage for ClansettingsDelta {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.affined as u8);
        buf.pdata(&self.data, 0, self.data.len());
    }

    fn sizeof(&self) -> usize {
        1 + self.data.len()
    }
}
