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

#[cfg(since_647)]
#[server_prot(MessageFriendchat, Buffered, VarByte)]
pub struct MessageFriendchat {
    pub sender: String,
    pub display_name: Option<String>,
    pub channel_owner: i64,
    pub msg_uid_hi: u16,
    pub msg_uid_lo: i32,
    pub tier: u8,
    pub message_data: Vec<u8>,
}

#[cfg(since_647)]
impl ServerProtMessage for MessageFriendchat {
    fn encode(&self, buf: &mut Packet) {
        if let Some(ref display) = self.display_name {
            buf.p1(1);
            buf.pjstr(&self.sender, 0);
            buf.pjstr(display, 0);
        } else {
            buf.p1(0);
            buf.pjstr(&self.sender, 0);
        }
        buf.p8(self.channel_owner);
        buf.p2(self.msg_uid_hi);
        buf.p3(self.msg_uid_lo);
        buf.p1(self.tier);
        buf.pdata(&self.message_data, 0, self.message_data.len());
    }

    fn sizeof(&self) -> usize {
        1 + jstr_len(&self.sender)
            + self.display_name.as_ref().map_or(0, |d| jstr_len(d))
            + size_of_val(&self.channel_owner)
            + size_of_val(&self.msg_uid_hi)
            + 3 // msg_uid_lo
            + size_of_val(&self.tier)
            + self.message_data.len()
    }
}
