use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(MessageClanchannel, Buffered, VarByte)]
pub struct MessageClanchannel {
    pub affined: bool,
    pub sender: String,
    pub msg_uid_hi: u16,
    pub msg_uid_lo: i32,
    pub tier: u8,
    pub message_data: Vec<u8>,
}

impl ServerProtMessage for MessageClanchannel {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(if self.affined { 1 } else { 0 });
        buf.pjstr(&self.sender, 0);
        buf.p2(self.msg_uid_hi);
        buf.p3(self.msg_uid_lo);
        buf.p1(self.tier);
        buf.pdata(&self.message_data, 0, self.message_data.len());
    }

    fn sizeof(&self) -> usize {
        1 + self.sender.len() + 1 + 2 + 3 + 1 + self.message_data.len()
    }
}
