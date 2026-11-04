//! `IfMovesub` — move a sub-interface from one parent component to another
//! without firing onload hooks. Rev-647 opcode 127, fixed 8.
//! Layout: `[p4_alt2 to_hash][p4_alt2 from_hash]`.
//!
//! Client read order (from decompiled rev-647 `MapAreaConfig`):
//! ```text
//! int to   = input_packet.g4_alt2();
//! int from = input_packet.g4_alt2();
//! ```

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(IfMovesub, Buffered, Fixed)]
pub struct IfMovesub {
    pub from_hash: i32,
    pub to_hash: i32,
}

impl ServerProtMessage for IfMovesub {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.to_hash);
        buf.p4_alt2(self.from_hash);
    }

    fn sizeof(&self) -> usize {
        8
    }
}
