//! `IfClosesub` — close a sub-interface by component hash.
//! Rev-647 opcode 74 (IF_CLOSESUB), fixed 4.
//! Layout: `[p4_alt3 component_hash]`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(IfClosesub, Buffered, Fixed)]
pub struct IfClosesub {
    pub component_hash: i32,
}

impl ServerProtMessage for IfClosesub {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt3(self.component_hash);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash)
    }
}
