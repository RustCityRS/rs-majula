//! `MessagePublic` — public chat broadcast.
//! Rev-647 opcode 117 (MESSAGE_PUBLIC), VarByte framing.
//!
//! Layout (from client-src decode):
//! ```text
//! p2   from_index     — speaker's player slot
//! p2   effects        — effect | (colour << 8), bit 15 = quickchat
//! p1   rights         — speaker's staff mod level
//! pSmart1or2  char_count  — decoded character count
//! pdata huffman_data  — Huffman-encoded CP-1252 bytes
//! ```

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(MessagePublic, Buffered, VarByte)]
pub struct MessagePublic {
    pub from_index: u16,
    pub effects: u16,
    pub rights: u8,
    pub char_count: usize,
    pub huffman_data: Vec<u8>,
}

impl ServerProtMessage for MessagePublic {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.from_index);
        buf.p2(self.effects);
        buf.p1(self.rights);
        buf.psmart1or2(self.char_count as i32);
        buf.pdata(&self.huffman_data, 0, self.huffman_data.len());
    }

    fn sizeof(&self) -> usize {
        let smart = if (self.char_count as i32) < 128 { 1 } else { 2 };
        2 + 2 + 1 + smart + self.huffman_data.len()
    }
}
