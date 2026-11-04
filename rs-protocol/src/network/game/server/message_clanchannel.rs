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
#[server_prot(MessageClanchannel, Buffered, VarByte)]
pub struct MessageClanchannel {
    pub affined: bool,
    pub sender: String,
    pub msg_uid_hi: u16,
    pub msg_uid_lo: i32,
    pub tier: u8,
    pub message_data: Vec<u8>,
}

#[cfg(since_647)]
impl ServerProtMessage for MessageClanchannel {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.affined as u8);
        buf.pjstr(&self.sender, 0);
        buf.p2(self.msg_uid_hi);
        buf.p3(self.msg_uid_lo);
        buf.p1(self.tier);
        buf.pdata(&self.message_data, 0, self.message_data.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.affined)
            + jstr_len(&self.sender)
            + size_of_val(&self.msg_uid_hi)
            + 3 // msg_uid_lo
            + size_of_val(&self.tier)
            + self.message_data.len()
    }
}
