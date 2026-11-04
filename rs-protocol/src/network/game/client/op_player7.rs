//! `OpPlayer7` — rev-647 OPPLAYER7 (opcode 64, fixed 3).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(3), UserEvent)]
pub struct OpPlayer7 {
    pub pid: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpPlayer7 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let (ctrl_run, pid) = (buf.g1_alt1() != 0, buf.g2());
        OpPlayer7 { pid, ctrl_run }
    }
}
