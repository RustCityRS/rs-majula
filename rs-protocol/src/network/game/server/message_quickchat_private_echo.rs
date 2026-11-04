use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(MessageQuickchatPrivateEcho, Buffered, VarByte)]
pub struct MessageQuickchatPrivateEcho {
    pub recipient: String,
    pub phrase_id: u16,
    pub quickchat_data: Vec<u8>,
}

impl ServerProtMessage for MessageQuickchatPrivateEcho {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.recipient, 0);
        buf.p2(self.phrase_id);
        buf.pdata(&self.quickchat_data, 0, self.quickchat_data.len());
    }

    fn sizeof(&self) -> usize {
        self.recipient.len() + 1 + 2 + self.quickchat_data.len()
    }
}
