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
#[server_prot(MessageQuickchatFriendchat, Buffered, VarByte)]
pub struct MessageQuickchatFriendchat<'a> {
    pub user37: i64,
    pub channel37: i64,
    pub id: i32,
    pub level: u8,
    pub phrase: TypeId,
    pub bytes: &'a [u8],
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for MessageQuickchatFriendchat<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p8(self.user37);
        buf.p1(0);
        buf.p8(self.channel37);
        buf.p2((self.id >> 24) as u16);
        buf.p3(self.id);
        buf.p1(self.level);
        buf.p2(self.phrase.g2());
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.user37)
            + 1
            + size_of_val(&self.channel37)
            + 5
            + size_of_val(&self.level)
            + size_of::<u16>()
            + self.bytes.len()
    }
}

#[cfg(since_647)]
#[server_prot(MessageQuickchatFriendchat, Buffered, VarByte)]
pub struct MessageQuickchatFriendchat {
    pub sender: String,
    pub display_name: Option<String>,
    pub channel37: i64,
    pub msg_uid_hi: u16,
    pub msg_uid_lo: i32,
    pub tier: u8,
    pub phrase_id: TypeId,
    pub quickchat_data: Vec<u8>,
}

#[cfg(since_647)]
impl ServerProtMessage for MessageQuickchatFriendchat {
    fn encode(&self, buf: &mut Packet) {
        if let Some(ref display) = self.display_name {
            buf.p1(1);
            buf.pjstr(&self.sender, STRING_TERMINATOR);
            buf.pjstr(display, STRING_TERMINATOR);
        } else {
            buf.p1(0);
            buf.pjstr(&self.sender, STRING_TERMINATOR);
        }
        buf.p8(self.channel37);
        buf.p2(self.msg_uid_hi);
        buf.p3(self.msg_uid_lo);
        buf.p1(self.tier);
        buf.p2(self.phrase_id.g2());
        buf.pdata(&self.quickchat_data, 0, self.quickchat_data.len());
    }

    fn sizeof(&self) -> usize {
        1 + Packet::sizejstr(&self.sender)
            + self.display_name.as_ref().map_or(0, |d| Packet::sizejstr(d))
            + size_of_val(&self.channel37)
            + size_of_val(&self.msg_uid_hi)
            + 3 // msg_uid_lo
            + size_of_val(&self.tier)
            + size_of::<u16>()
            + self.quickchat_data.len()
    }
}
