//! `CamRemoveroof` — toggle roof removal around a focal coord.
//! Rev-647 opcode 36 (CAM_REMOVEROOF), fixed 4.
//!
//! Note: the rev-647 wire uses a packed 32-bit word, **not** a
//! single-byte toggle. `coord_hash` = -1 clears the focal point
//! (client resets `anInt173` / `anInt2167` to -1), otherwise the
//! client unpacks `x = (h >> 14) & 0x3FFF`, `z = h & 0x3FFF`.
//!
//! Client read order (`MapAreaConfig.java`):
//! `g4_alt2 coord_hash` (single packed 32-bit word).

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(CamRemoveroof, Buffered, Fixed)]
pub struct CamRemoveroof {
    /// Packed `(x << 14) | z`, or -1 to clear.
    pub coord_hash: i32,
}

impl ServerProtMessage for CamRemoveroof {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.coord_hash);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.coord_hash)
    }
}
