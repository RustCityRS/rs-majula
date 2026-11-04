//! `IfSetTargetParam` — set drag/use-target params for an interface
//! component (which inventory it can target, slot range).
//!
//! Rev-647 opcode 79 (IF_SETTARGETPARAM), fixed 10. ⚠️ Rev-647-specific.
//! Layout: `[ip2 from][ip2_alt1 inv_id][p2_alt1 to][p4_alt3 component_hash]`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(IfSetTargetParam, Buffered, Fixed)]
pub struct IfSetTargetParam {
    pub component_hash: i32,
    pub from_slot: i32,
    pub to_slot: i32,
    pub inv_id: i32,
}

impl ServerProtMessage for IfSetTargetParam {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.from_slot as u16);
        buf.ip2_alt1(self.inv_id as u16);
        buf.p2_alt1(self.to_slot as u16);
        buf.p4_alt3(self.component_hash);
    }

    fn sizeof(&self) -> usize {
        10
    }
}
