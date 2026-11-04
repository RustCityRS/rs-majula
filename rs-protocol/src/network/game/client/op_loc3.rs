//! `OpLoc3` — rev-647 OPLOC3 (opcode 36, fixed 7).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(7), UserEvent)]
pub struct OpLoc3 {
    pub loc_id: u16,
    pub x: u16,
    pub z: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpLoc3 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2_alt1();
        let ctrl_run = buf.g1_alt2() != 0;
        let z = buf.ig2();
        let loc_id = buf.g2();
        OpLoc3 {
            loc_id,
            x,
            z,
            ctrl_run,
        }
    }
}
