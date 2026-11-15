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

#[cfg(all(since_530, before_647))]
#[server_prot(MessagePrivateEcho, Immediate, VarByte)]
pub struct MessagePrivateEcho<'a> {
    pub user37: i64,
    pub bytes: &'a [u8],
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for MessagePrivateEcho<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p8(self.user37);
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.user37) + self.bytes.len()
    }
}

#[cfg(since_647)]
#[cfg_attr(before_742, server_prot(MessagePrivateEcho, Immediate, VarByte))]
#[cfg_attr(since_742, server_prot(MessagePrivateEcho, Immediate, VarShort))]
pub struct MessagePrivateEcho {
    pub recipient: String,
    pub message_data: Vec<u8>,
}

#[cfg(since_647)]
impl ServerProtMessage for MessagePrivateEcho {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.recipient, STRING_TERMINATOR);
        buf.pdata(&self.message_data, 0, self.message_data.len());
    }

    fn sizeof(&self) -> usize {
        Packet::sizejstr(&self.recipient) + self.message_data.len()
    }
}
