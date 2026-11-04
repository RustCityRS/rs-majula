//! `VarcLarge` — update a client var (varc) with a value outside `-128..=127`.
//! Rev-647 wire opcode 98 (`CLIENT_SETVARC_LARGE`).
//! Layout: `[ip4 value][ip2_alt1 varc]` — fixed 6 bytes.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(VarcLarge, Buffered, Fixed)]
pub struct VarcLarge {
    pub varc: u16,
    pub value: i32,
}

impl ServerProtMessage for VarcLarge {
    fn encode(&self, buf: &mut Packet) {
        buf.ip4(self.value);
        buf.ip2_alt1(self.varc);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.varc) + size_of_val(&self.value)
    }
}
