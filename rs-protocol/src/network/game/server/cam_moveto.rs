//! `CamMoveto` — move the camera position to a local tile.
//! Rev-647 opcode 94 (CAM_MOVETO), fixed 6.
//! Layout:
//!   `[p1_alt2 local_z][p1_alt2 const_speed][p1_alt1 local_x][p1_alt1 var_speed][p2_alt1 height]`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(CamMoveto, Buffered, Fixed)]
pub struct CamMoveto {
    pub local_x: u8,
    pub local_z: u8,
    pub height: u16,
    pub const_speed: u8,
    pub var_speed: u8,
}

impl ServerProtMessage for CamMoveto {
    fn encode(&self, buf: &mut Packet) {
        // `MapAreaConfig`'s CAM_MOVETO branch, into `Class36.method269`.
        // Height in 1/128-tile units, scaled by the client as for CAM_LOOKAT.
        buf.p1_alt3(self.local_x);
        buf.p1_alt1(self.local_z);
        buf.p1_alt3(self.var_speed);
        buf.p2(self.height);
        buf.p1_alt3(self.const_speed);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.local_x)
            + size_of_val(&self.local_z)
            + size_of_val(&self.height)
            + size_of_val(&self.const_speed)
            + size_of_val(&self.var_speed)
    }
}
