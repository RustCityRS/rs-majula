//! `OpNpc3` — rev-647 OPNPC3 (opcode 26, fixed 3).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(3), UserEvent)]
pub struct OpNpc3 {
    pub nid: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpNpc3 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let (ctrl_run, nid) = (buf.g1_alt2() != 0, buf.g2_alt1());
        OpNpc3 { nid, ctrl_run }
    }
}
