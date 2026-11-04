//! `LocAnimSpecific` — standalone loc animation packet (distinct from the
//! zone sub-packet `LOC_ANIM`).
//! Rev-647 opcode 68 (LOC_ANIM_SPECIFIC), fixed 7.
//!
//! Client read order (`MapAreaConfig.java`):
//! `ig2 seq_id; g1_alt2 shape_rotation; g4 coord_hash`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(LocAnimSpecific, Buffered, Fixed)]
pub struct LocAnimSpecific {
    /// Sequence id (65535 → -1 sentinel).
    pub seq_id: u16,
    /// Packed `shape<<2 | rotation`. Client splits `shape = byte >> 2`,
    /// `rotation = byte & 0x3`.
    pub shape_rotation: u8,
    /// Packed coord word: `(level<<28) | (x<<14) | z`.
    pub coord_hash: i32,
}

impl ServerProtMessage for LocAnimSpecific {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.seq_id);
        buf.p1_alt2(self.shape_rotation);
        buf.p4(self.coord_hash);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.seq_id)
            + size_of_val(&self.shape_rotation)
            + size_of_val(&self.coord_hash)
    }
}
