//! `IfSetEvents` — enable interface component events (clicks, drag, etc.).
//!
//! Rev-647 opcode 57, fixed 12. ⚠️ Rev-647-specific — 2004scape has no
//! equivalent packet (that revision used `SET_PLAYER_OP` + varp masks
//! instead).
//!
//! Layout:
//!   `[p2_alt1 from_slot][p4_alt3 settings][p4_alt3 component_hash][ip2_alt1 to_slot]`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(IfSetEvents, Buffered, Fixed)]
pub struct IfSetEvents {
    pub component_hash: i32,
    pub from_slot: i32,
    pub to_slot: i32,
    pub settings: i32,
}

impl ServerProtMessage for IfSetEvents {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.from_slot as u16);
        buf.p4_alt3(self.settings);
        buf.p4_alt3(self.component_hash);
        buf.ip2_alt1(self.to_slot as u16);
    }

    fn sizeof(&self) -> usize {
        12
    }
}
