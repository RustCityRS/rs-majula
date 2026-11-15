#[cfg(since_530)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_530)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_530)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_530)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_530)]
use rs_protocol_macros::server_prot;
#[cfg(since_647)]
use rs_ver::STRING_TERMINATOR;
use rs_ver::TypeId;

#[cfg(all(since_530, before_647))]
#[server_prot(MessageQuickchatPrivateEcho, Buffered, VarByte)]
pub struct MessageQuickchatPrivateEcho<'a> {
    pub user37: i64,
    pub phrase: TypeId,
    pub bytes: &'a [u8],
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for MessageQuickchatPrivateEcho<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p8(self.user37);
        buf.p2(self.phrase.g2());
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.user37) + size_of::<u16>() + self.bytes.len()
    }
}

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
