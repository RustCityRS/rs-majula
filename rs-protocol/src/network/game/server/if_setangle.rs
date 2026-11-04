//! `IfSetAngle` — set an interface component model's angle/zoom.
//! Rev-647 opcode 56 (IF_SETANGLE), fixed 10.
//!
//! Client read order (`MapAreaConfig.java`):
//! `ig2 x_rotation; g4_alt1 component_hash; g2_alt1 y_rotation; g2_alt1 zoom`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(IfSetAngle, Buffered, Fixed)]
pub struct IfSetAngle {
    /// Packed `(interface_id << 16) | component_id`.
    pub component_hash: i32,
    /// Rotation around the X axis.
    pub x_rotation: u16,
    /// Rotation around the Y axis.
    pub y_rotation: u16,
    /// Model zoom factor.
    pub zoom: u16,
}

impl ServerProtMessage for IfSetAngle {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.x_rotation);
        buf.p4_alt1(self.component_hash);
        buf.p2_alt1(self.y_rotation);
        buf.p2_alt1(self.zoom);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash)
            + size_of_val(&self.x_rotation)
            + size_of_val(&self.y_rotation)
            + size_of_val(&self.zoom)
    }
}
