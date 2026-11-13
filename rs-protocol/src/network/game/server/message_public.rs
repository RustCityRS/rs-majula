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
#[server_prot(MessagePublic, Buffered, VarByte)]
pub struct MessagePublic<'a> {
    pub pid: u16,
    pub chat: &'a [u8],
}

#[cfg(since_647)]
impl ServerProtMessage for MessagePublic<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.pid);
        buf.pdata(self.chat, 0, self.chat.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.pid) + self.chat.len()
    }
}
