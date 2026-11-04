//! `OpPlayer5` — rev-647 OPPLAYER5 (opcode 20, fixed 3).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(3), UserEvent)]
pub struct OpPlayer5 {
    pub pid: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpPlayer5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let (ctrl_run, pid) = (buf.g1_alt3() != 0, buf.ig2());
        OpPlayer5 { pid, ctrl_run }
    }
}
