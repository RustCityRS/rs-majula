use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(CreateCheckName, Buffered, Fixed)]
pub struct CreateCheckName {
    pub response: u8,
}

impl ServerProtMessage for CreateCheckName {
    // Client reads `g1b_alt1` (= `-byte & 0xFF`), not a plain byte
    // (`MapAreaConfig.java`, CREATE_CHECK_NAME).
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt1(self.response);
    }

    fn sizeof(&self) -> usize {
        1
    }
}
