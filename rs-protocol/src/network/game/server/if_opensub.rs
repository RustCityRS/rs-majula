//! `IfOpensub` — open a sub-interface inside a parent component.
//!
//! Rev-647 opcode 38 (IF_OPENSUB), fixed 7. The rev-647 protocol uses
//! this single packet for all sub-interface opens (chat, main modal,
//! side panel, tab sub) — 2004scape's separate `IF_OPENCHAT` /
//! `IF_OPENMAIN` / `IF_OPENSIDE` are folded into this one by the
//! client's component hash convention.
//!
//! Layout: `[ip2_alt1 sub_id][p4_alt1 parent_hash][p1_alt3 walkable]`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(IfOpensub, Buffered, Fixed)]
pub struct IfOpensub {
    pub sub_id: u16,
    pub parent_hash: i32,
    pub walkable: u8,
}

impl ServerProtMessage for IfOpensub {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2_alt1(self.sub_id);
        buf.p4_alt1(self.parent_hash);
        buf.p1_alt3(self.walkable);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.sub_id) + size_of_val(&self.parent_hash) + size_of_val(&self.walkable)
    }
}
