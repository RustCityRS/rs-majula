use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(ChatFilterSettingsPrivatechat, Buffered, Fixed)]
pub struct ChatFilterSettingsPrivatechat {
    pub private_filter: u8,
}

impl ServerProtMessage for ChatFilterSettingsPrivatechat {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.private_filter);
    }

    fn sizeof(&self) -> usize {
        1
    }
}
