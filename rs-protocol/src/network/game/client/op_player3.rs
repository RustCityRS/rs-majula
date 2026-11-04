//! `OpPlayer3` — rev-647 OPPLAYER3 (opcode 30, fixed 3).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(3), UserEvent)]
pub struct OpPlayer3 {
    pub pid: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpPlayer3 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let pid = buf.ig2();
        let ctrl_run = buf.g1() != 0;
        OpPlayer3 { pid, ctrl_run }
    }
}
