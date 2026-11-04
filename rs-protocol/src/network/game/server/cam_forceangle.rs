//! `CamForceangle` — force the camera to a specific angle.
//! Rev-647 opcode 29 (CAM_FORCEANGLE), fixed 4.
//!
//! Client read order (`MapAreaConfig.java`):
//! `ig2 yaw; g2_alt1 pitch`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(CamForceangle, Buffered, Fixed)]
pub struct CamForceangle {
    /// Yaw/horizontal angle (client `var270`).
    pub yaw: u16,
    /// Pitch/vertical angle (client `var230`).
    pub pitch: u16,
}

impl ServerProtMessage for CamForceangle {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.yaw);
        buf.p2_alt1(self.pitch);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.yaw) + size_of_val(&self.pitch)
    }
}
