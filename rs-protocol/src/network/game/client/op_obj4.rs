//! `OpObj4` — rev-647 OPOBJ4 (opcode 44, fixed 7).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(7), UserEvent)]
pub struct OpObj4 {
    pub obj_id: u16,
    pub x: u16,
    pub z: u16,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpObj4 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.ig2();
        let z = buf.g2();
        let ctrl_run = buf.g1_alt2() != 0;
        let obj_id = buf.ig2();
        OpObj4 {
            obj_id,
            x,
            z,
            ctrl_run,
        }
    }
}
