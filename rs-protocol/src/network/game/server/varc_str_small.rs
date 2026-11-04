//! `VarcStrSmall` — set a client string var (varc_str) on the client.
//! Rev-647 wire opcode 81 (`CLIENT_SETVARCSTR_SMALL`).
//! Layout: `[pjstr value][ig2_alt1 varc]` — var byte framing.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(VarcStrSmall, Buffered, VarByte)]
pub struct VarcStrSmall {
    pub varc: u16,
    pub value: String,
}

impl ServerProtMessage for VarcStrSmall {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.value, 0);
        buf.ip2_alt1(self.varc);
    }

    fn sizeof(&self) -> usize {
        self.value.len() + 1 + 2
    }
}
