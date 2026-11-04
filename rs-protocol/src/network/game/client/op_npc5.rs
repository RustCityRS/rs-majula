//! `OpNpc5` — rev-647 OPNPC5 (opcode 79, fixed 3).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(3), UserEvent)]
pub struct OpNpc5 {
    pub nid: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpNpc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let (ctrl_run, nid) = (buf.g1_alt1() != 0, buf.ig2());
        OpNpc5 { nid, ctrl_run }
    }
}
