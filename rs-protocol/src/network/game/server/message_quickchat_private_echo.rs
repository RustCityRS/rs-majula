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
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;

#[cfg(since_647)]
#[server_prot(MessageQuickchatPrivateEcho, Buffered, VarByte)]
pub struct MessageQuickchatPrivateEcho {
    pub recipient: String,
    pub phrase_id: TypeId,
    pub quickchat_data: Vec<u8>,
}

#[cfg(since_647)]
impl ServerProtMessage for MessageQuickchatPrivateEcho {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.recipient, STRING_TERMINATOR);
        buf.p2(self.phrase_id.g2());
        buf.pdata(&self.quickchat_data, 0, self.quickchat_data.len());
    }

    fn sizeof(&self) -> usize {
        Packet::sizejstr(&self.recipient) + size_of::<u16>() + self.quickchat_data.len()
    }
}
