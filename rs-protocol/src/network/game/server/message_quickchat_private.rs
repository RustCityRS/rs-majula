use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

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

impl ServerProtMessage for MessageQuickchatPrivate {
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
        buf.p2(self.phrase_id);
        buf.pdata(&self.quickchat_data, 0, self.quickchat_data.len());
    }

    fn sizeof(&self) -> usize {
        let name_len =
            1 + self.sender.len() + 1 + self.display_name.as_ref().map_or(0, |d| d.len() + 1);
        name_len + 2 + 3 + 1 + 2 + self.quickchat_data.len()
    }
}
