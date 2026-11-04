use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(CamReset, Buffered, Fixed)]
pub struct CamReset;

#[cfg(before_647)]
impl ServerProtMessage for CamReset {
    fn encode(&self, _: &mut Packet) {}

    fn sizeof(&self) -> usize {
        0
    }
}

// `CamReset` — reset the camera to its default position/angle.
// Rev-647 opcode 67 (CAM_RESET), fixed 0 (empty payload).

#[cfg(since_647)]
#[server_prot(CamReset, Buffered, Fixed)]
pub struct CamReset;

#[cfg(since_647)]
impl ServerProtMessage for CamReset {
    fn encode(&self, _buf: &mut Packet) {}

    fn sizeof(&self) -> usize {
        0
    }
}
