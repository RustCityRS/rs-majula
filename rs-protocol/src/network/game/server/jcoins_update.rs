//! `JcoinsUpdate` — broadcast the player's current JCoins balance.
//! Rev-647 opcode 19 (JCOINS_UPDATE), fixed 4.
//!
//! Client read order (`MapAreaConfig.java`):
//! `ig4 balance`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(JcoinsUpdate, Buffered, Fixed)]
pub struct JcoinsUpdate {
    pub balance: i32,
}

impl ServerProtMessage for JcoinsUpdate {
    fn encode(&self, buf: &mut Packet) {
        buf.ip4(self.balance);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.balance)
    }
}
