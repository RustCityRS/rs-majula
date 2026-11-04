use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(MessagePrivateEcho, Buffered, VarByte)]
pub struct MessagePrivateEcho {
    pub recipient: String,
    pub message_data: Vec<u8>,
}

impl ServerProtMessage for MessagePrivateEcho {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.recipient, 0);
        buf.pdata(&self.message_data, 0, self.message_data.len());
    }

    fn sizeof(&self) -> usize {
        self.recipient.len() + 1 + self.message_data.len()
    }
}
