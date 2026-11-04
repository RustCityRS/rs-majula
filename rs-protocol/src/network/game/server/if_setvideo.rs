//! `IfSetVideo` — attach a video stream to an interface component.
//! Rev-647 opcode 65 (IF_SETVIDEO), fixed 6.
//!
//! Client read order (`MapAreaConfig.java`):
//! `ig2 video_id; g4 component_hash`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(IfSetVideo, Buffered, Fixed)]
pub struct IfSetVideo {
    /// Video-source id.
    pub video_id: u16,
    /// Packed `(interface_id << 16) | component_id`.
    pub component_hash: i32,
}

impl ServerProtMessage for IfSetVideo {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.video_id);
        buf.p4(self.component_hash);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.video_id) + size_of_val(&self.component_hash)
    }
}
