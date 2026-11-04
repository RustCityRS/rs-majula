//! `ExamineLoc` — rev-647 OPLOC6 (opcode 31, fixed 2).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(2), UserEvent)]
pub struct ExamineLoc {
    pub loc_id: u16,
}

impl ClientProtMessage for ExamineLoc {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ExamineLoc { loc_id: buf.g2() }
    }
}
