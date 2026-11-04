//! `OpPlayer8` — rev-647 OPPLAYER8 (opcode 70, fixed 3).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(3), UserEvent)]
pub struct OpPlayer8 {
    pub pid: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpPlayer8 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let (ctrl_run, pid) = (buf.g1() != 0, buf.ig2());
        OpPlayer8 { pid, ctrl_run }
    }
}
