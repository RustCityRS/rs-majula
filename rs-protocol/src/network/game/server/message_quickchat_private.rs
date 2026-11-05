#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::jstr_len;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;
use rs_ver::STRING_TERMINATOR;

#[cfg(since_647)]
#[server_prot(MessageQuickchatPrivate, Buffered, VarByte)]
pub struct MessageQuickchatPrivate {
    pub sender: String,
    pub display_name: Option<String>,
    pub msg_uid_hi: u16,
    pub msg_uid_lo: i32,
    pub tier: u8,
    pub phrase_id: u16,
    pub quickchat_data: Vec<u8>,
}

#[cfg(since_647)]
impl ServerProtMessage for MessageQuickchatPrivate {
    fn encode(&self, buf: &mut Packet) {
        if let Some(ref display) = self.display_name {
            buf.p1(1);
            buf.pjstr(&self.sender, STRING_TERMINATOR);
            buf.pjstr(display, STRING_TERMINATOR);
        } else {
            buf.p1(0);
            buf.pjstr(&self.sender, STRING_TERMINATOR);
        }
        buf.p2(self.msg_uid_hi);
        buf.p3(self.msg_uid_lo);
        buf.p1(self.tier);
        buf.p2(self.phrase_id);
        buf.pdata(&self.quickchat_data, 0, self.quickchat_data.len());
    }

    fn sizeof(&self) -> usize {
        1 + jstr_len(&self.sender)
            + self.display_name.as_ref().map_or(0, |d| jstr_len(d))
            + size_of_val(&self.msg_uid_hi)
            + 3 // msg_uid_lo
            + size_of_val(&self.tier)
            + size_of_val(&self.phrase_id)
            + self.quickchat_data.len()
    }
}
