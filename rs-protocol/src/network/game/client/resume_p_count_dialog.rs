//! `ResumePCountDialog` — rev-647 RESUME_P_COUNTDIALOG (opcode 35, fixed 4).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(4), UserEvent)]
pub struct ResumePCountDialog {
    pub value: i32,
}

impl ClientProtMessage for ResumePCountDialog {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ResumePCountDialog { value: buf.g4s() }
    }
}
