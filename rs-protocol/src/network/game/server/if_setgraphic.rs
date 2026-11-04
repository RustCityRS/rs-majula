//! `IfSetGraphic` — set an interface component's model graphic.
//! Rev-647 opcode 55 (IF_SETGRAPHIC), fixed 6.
//!
//! Client read order (`MapAreaConfig.java`):
//! `ig4 component_hash; g2_alt1 graphic_id`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(IfSetGraphic, Buffered, Fixed)]
pub struct IfSetGraphic {
    /// Packed `(interface_id << 16) | component_id`.
    pub component_hash: i32,
    /// Sprite/graphic id. 65535 → -1 clears the graphic.
    pub graphic_id: u16,
}

impl ServerProtMessage for IfSetGraphic {
    fn encode(&self, buf: &mut Packet) {
        buf.ip4(self.component_hash);
        buf.p2_alt1(self.graphic_id);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash) + size_of_val(&self.graphic_id)
    }
}
