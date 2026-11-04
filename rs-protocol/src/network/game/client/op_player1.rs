//! `OpPlayer1` — rev-647 OPPLAYER1 (opcode 75, fixed 3).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(3), UserEvent)]
pub struct OpPlayer1 {
    pub pid: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpPlayer1 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let (ctrl_run, pid) = (buf.g1_alt3() != 0, buf.g2_alt1());
        OpPlayer1 { pid, ctrl_run }
    }
}
