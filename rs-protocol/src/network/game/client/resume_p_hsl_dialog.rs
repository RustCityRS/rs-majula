//! `ResumePHslDialog` — rev-647 RESUME_P_HSLDIALOG (opcode 12, fixed 2).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(2), UserEvent)]
pub struct ResumePHslDialog {
    pub hsl: u16,
}

impl ClientProtMessage for ResumePHslDialog {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ResumePHslDialog { hsl: buf.g2() }
    }
}
