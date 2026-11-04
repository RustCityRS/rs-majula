//! `CamLookat` — turn the camera to look at a local tile.
//! Rev-647 opcode 26 (CAM_LOOKAT), fixed 6.
//! Layout: `[p1 local_x][p1_alt2 local_z][p1 const_speed][ip2 height][p1_alt1 var_speed]`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(CamLookat, Buffered, Fixed)]
pub struct CamLookat {
    pub local_x: u8,
    pub local_z: u8,
    pub height: u16,
    pub const_speed: u8,
    pub var_speed: u8,
}

impl ServerProtMessage for CamLookat {
    fn encode(&self, buf: &mut Packet) {
        // `MapAreaConfig`'s CAM_LOOKAT branch, into `AClass13.method622`.
        // Height is in 1/128-tile units, as before 647; the client scales it
        // (`<< 2`) to its 512-unit tiles.
        buf.p1_alt1(self.local_x);
        buf.ip2_alt1(self.height);
        buf.p1_alt2(self.local_z);
        buf.p1_alt1(self.const_speed);
        buf.p1_alt2(self.var_speed);
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
