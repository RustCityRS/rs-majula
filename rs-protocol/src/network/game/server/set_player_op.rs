#[cfg(since_254)]
use crate::network::game::server::ServerProtMessage;
#[cfg(since_254)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_254)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_254)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_254)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_254)]
use rs_protocol_macros::server_prot;

#[cfg(since_254)]
#[server_prot(SetPlayerOp, Buffered, VarByte)]
pub struct SetPlayerOp<'a> {
    pub op: u8,
    pub value: &'a str,
    pub primary: u8,
}

#[cfg(since_254)]
impl ServerProtMessage for SetPlayerOp<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.op);
        buf.p1(self.primary);
        buf.pjstr(self.value, 10);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.op) + size_of_val(&self.primary) + self.value.len() + 1
    }
}
