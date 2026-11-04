//! `IfButton5` — rev-647 IF_BUTTON5 (opcode 83, fixed 8).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(8), UserEvent)]
pub struct IfButton5 {
    pub com_hash: u32,
    pub slot: i16,
    pub obj: i16,
}

impl ClientProtMessage for IfButton5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com_hash = buf.g4s() as u32;
        let obj = buf.ig2_alt1() as i16;
        let slot = buf.ig2() as i16;
        IfButton5 {
            com_hash,
            slot,
            obj,
        }
    }
}
