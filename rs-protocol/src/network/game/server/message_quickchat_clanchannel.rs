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
#[server_prot(MessageQuickchatClanchannel, Buffered, VarByte)]
pub struct MessageQuickchatClanchannel {
    pub affined: bool,
    pub sender: String,
    pub msg_uid_hi: u16,
    pub msg_uid_lo: i32,
    pub tier: u8,
    pub phrase_id: u16,
    pub quickchat_data: Vec<u8>,
}

#[cfg(since_647)]
impl ServerProtMessage for MessageQuickchatClanchannel {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.affined as u8);
        buf.pjstr(&self.sender, 0);
        buf.p2(self.msg_uid_hi);
        buf.p3(self.msg_uid_lo);
        buf.p1(self.tier);
        buf.p2(self.phrase_id);
        buf.pdata(&self.quickchat_data, 0, self.quickchat_data.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.affined)
            + jstr_len(&self.sender)
            + size_of_val(&self.msg_uid_hi)
            + 3 // msg_uid_lo
            + size_of_val(&self.tier)
            + size_of_val(&self.phrase_id)
            + self.quickchat_data.len()
    }
}
