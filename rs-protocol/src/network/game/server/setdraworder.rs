//! `Setdraworder` — UI layer ordering flag.
//! Rev-647 opcode 100 (SETDRAWORDER), fixed 1.
//!
//! Client read order (`MapAreaConfig.java`):
//! `g1_alt1 mode`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(Setdraworder, Buffered, Fixed)]
pub struct Setdraworder {
    pub mode: u8,
}

impl ServerProtMessage for Setdraworder {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt1(self.mode);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.mode)
    }
}
