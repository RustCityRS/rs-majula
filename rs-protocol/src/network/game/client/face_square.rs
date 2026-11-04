//! `FaceSquare` — rev-647 opcode 5 (fixed 4): the "Face here" menu option.
//!
//! `Statics1.do_action`, `MiniMenuAction.FACE_SQUARE`: `p2_alt1(x) ip2(z)`,
//! absolute tile coordinates. The client adds the option only once the
//! server has turned it on with SHOW_FACE_HERE (or when the game is Stellar
//! Dawn); staff holding ctrl+shift teleport there locally instead.
use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(4), UserEvent)]
pub struct FaceSquare {
    pub x: u16,
    pub z: u16,
}

impl ClientProtMessage for FaceSquare {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2_alt1();
        let z = buf.ig2();
        FaceSquare { x, z }
    }
}
