//! `ApCoordT` — rev-647 APCOORDT (opcode 74, fixed 12): the selected item or
//! spell used on a ground square.
//!
//! Client write order (`Statics1.do_action`, `MiniMenuAction.TGT_GROUND`),
//! the source fields as in `OpLocT`:
//! ```text
//! ip2(region_x + arg1)     // TARGET tile x   -> x
//! p2(anInt5794)            // SOURCE slot     -> from_slot
//! p2_alt1(anInt123)        // SOURCE obj id   -> from_obj
//! p2_alt1(region_z + arg2) // TARGET tile z   -> z
//! p4(anInt2320)            // SOURCE com      -> from_com
//! ```

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(12), UserEvent)]
pub struct ApCoordT {
    pub x: u16,
    pub z: u16,
    pub from_obj: u16,
    pub from_slot: u16,
    pub from_com: u32,
}

impl ClientProtMessage for ApCoordT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.ig2();
        let from_slot = buf.g2();
        let from_obj = buf.g2_alt1();
        let z = buf.g2_alt1();
        let from_com = buf.g4s() as u32;
        ApCoordT {
            x,
            z,
            from_obj,
            from_slot,
            from_com,
        }
    }
}
