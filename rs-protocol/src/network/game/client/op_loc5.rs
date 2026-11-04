//! `OpLoc5` — rev-647 OPLOC5 (opcode 48, fixed 7).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(7), UserEvent)]
pub struct OpLoc5 {
    pub loc_id: u16,
    pub x: u16,
    pub z: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpLoc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let loc_id = buf.g2_alt1();
        let x = buf.g2_alt1();
        let ctrl_run = buf.g1_alt3() != 0;
        let z = buf.ig2();
        OpLoc5 {
            loc_id,
            x,
            z,
            ctrl_run,
        }
    }
}
