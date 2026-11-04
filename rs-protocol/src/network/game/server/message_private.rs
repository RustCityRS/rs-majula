use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_647)]
use crate::network::game::server_prot_message::jstr_len;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(MessagePrivate, Immediate, VarByte)]
pub struct MessagePrivate<'a> {
    pub user37: i64,
    pub id: i32,
    pub level: u8,
    pub bytes: &'a [u8],
}

#[cfg(before_647)]
impl ServerProtMessage for MessagePrivate<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p8(self.user37);
        buf.p4(self.id);
        buf.p1(self.level);
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.user37)
            + size_of_val(&self.id)
            + size_of_val(&self.level)
            + self.bytes.len()
    }
}

#[cfg(since_647)]
#[server_prot(MessagePrivate, Immediate, VarByte)]
pub struct MessagePrivate {
    pub sender: String,
    pub display_name: Option<String>,
    pub msg_uid_hi: u16,
    pub msg_uid_lo: i32,
    pub tier: u8,
    pub message_data: Vec<u8>,
}

#[cfg(since_647)]
impl ServerProtMessage for MessagePrivate {
    fn encode(&self, buf: &mut Packet) {
        if let Some(ref display) = self.display_name {
            buf.p1(1);
            buf.pjstr(&self.sender, 0);
            buf.pjstr(display, 0);
        } else {
            buf.p1(0);
            buf.pjstr(&self.sender, 0);
        }
        buf.p2(self.msg_uid_hi);
        buf.p3(self.msg_uid_lo);
        buf.p1(self.tier);
        buf.pdata(&self.message_data, 0, self.message_data.len());
    }

    fn sizeof(&self) -> usize {
        1 + jstr_len(&self.sender)
            + self.display_name.as_ref().map_or(0, |d| jstr_len(d))
            + size_of_val(&self.msg_uid_hi)
            + 3 // msg_uid_lo
            + size_of_val(&self.tier)
            + self.message_data.len()
    }
}
