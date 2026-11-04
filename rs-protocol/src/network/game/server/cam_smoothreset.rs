//! `CamSmoothreset` — rev-647 CAM_SMOOTHRESET (opcode 12, fixed 0).
//!
//! Before this was named `PingReply` and documented as a keep-alive. The
//! client's branch for opcode 12 resets the camera
//! (`MapAreaConfig`, `ServerProt.CAM_SMOOTHRESET`), so sending it as a
//! keep-alive would have snapped the camera back every time. Nothing sends it.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(CamSmoothreset, Buffered, Fixed)]
pub struct CamSmoothreset;

impl ServerProtMessage for CamSmoothreset {
    fn encode(&self, _buf: &mut Packet) {}

    fn sizeof(&self) -> usize {
        0
    }
}
