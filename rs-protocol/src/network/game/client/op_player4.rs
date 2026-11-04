//! `OpPlayer4` — rev-647 OPPLAYER4 (opcode 22, fixed 3).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(3), UserEvent)]
pub struct OpPlayer4 {
    pub pid: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpPlayer4 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let pid = buf.g2_alt1();
        let ctrl_run = buf.g1() != 0;
        OpPlayer4 { pid, ctrl_run }
    }
}
