//! `VarcSmall` — update a client var (varc) with a value in `-128..=127`.
//! Rev-647 wire opcode 52 (`CLIENT_SETVARC_SMALL`).
//! Layout: `[p1_alt1 value][ip2 varc]` — fixed 3 bytes.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(VarcSmall, Buffered, Fixed)]
pub struct VarcSmall {
    pub varc: u16,
    pub value: i32,
}

impl ServerProtMessage for VarcSmall {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt1(self.value as u8);
        buf.ip2(self.varc);
    }

    fn sizeof(&self) -> usize {
        // u8 value byte + u16 varc = 3
        3
    }
}
