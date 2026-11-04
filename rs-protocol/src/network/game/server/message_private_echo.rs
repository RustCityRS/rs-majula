#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{
    ServerProtMessage, ServerProtMessageInfo, jstr_len,
};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;

#[cfg(since_647)]
#[server_prot(MessagePrivateEcho, Immediate, VarByte)]
pub struct MessagePrivateEcho {
    pub recipient: String,
    pub message_data: Vec<u8>,
}

#[cfg(since_647)]
impl ServerProtMessage for MessagePrivateEcho {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.recipient, 0);
        buf.pdata(&self.message_data, 0, self.message_data.len());
    }

    fn sizeof(&self) -> usize {
        jstr_len(&self.recipient) + self.message_data.len()
    }
}
