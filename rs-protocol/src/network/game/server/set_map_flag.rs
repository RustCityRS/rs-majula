//! `SetMapFlag` — set/clear the yellow map-click flag on the minimap.
//! Rev-647 opcode 110 (SET_MAP_FLAG), fixed 2.
//! Layout: `[p1_alt2 x][p1_alt2 z]`. Send `(255, 255)` to clear.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(SetMapFlag, Buffered, Fixed)]
pub struct SetMapFlag {
    pub x: u8,
    pub z: u8,
}

impl ServerProtMessage for SetMapFlag {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.x);
        buf.p1_alt2(self.z);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.x) + size_of_val(&self.z)
    }
}
