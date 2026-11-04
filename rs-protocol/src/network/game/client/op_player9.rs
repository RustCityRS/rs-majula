//! `OpPlayer9` — rev-647 OPPLAYER9 (opcode 9, fixed 3).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(3), UserEvent)]
pub struct OpPlayer9 {
    pub pid: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpPlayer9 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let pid = buf.g2();
        let ctrl_run = buf.g1_alt3() != 0;
        OpPlayer9 { pid, ctrl_run }
    }
}
