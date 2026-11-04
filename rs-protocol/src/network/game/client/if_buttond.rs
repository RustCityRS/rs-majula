//! `IfButtonD` — rev-647 IF_BUTTOND (opcode 37, fixed 16).
//!
//! Field order from the client's own writer, `SceneGraphNode_Sub6_Sub1.method3715`.
//! Note its parameters: `method3715(arg0 = the component released over, arg1 =
//! the component picked up)` — so `arg1` is the SOURCE and `arg0` is the TARGET,
//! and it writes the source first:
//!
//! ```text
//!   p4(arg1.id)              source component
//!   ip2_alt1(arg1.child_id)  source slot
//!   ip2_alt1(arg0.child_id)  target slot
//!   p4_alt2(arg0.id)         target component
//!   ip2(arg1.object_id)      source obj
//!   p2(arg0.object_id)       target obj
//! ```
//!
//! This was originally decoded with from/to the other way round. A same-inventory
//! drag hid the mistake, because moving slot A onto slot B is a swap and a swap is
//! symmetric — it only shows up once the two endpoints differ, i.e. any drag
//! between two containers.

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(16), UserEvent)]
pub struct IfButtonD {
    pub from_com: u32,
    pub from_slot: u16,
    pub from_obj: u16,
    pub to_com: u32,
    pub to_slot: u16,
    pub to_obj: u16,
}

impl ClientProtMessage for IfButtonD {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let from_com = buf.g4s() as u32;
        let from_slot = buf.ig2_alt1();
        let to_slot = buf.ig2_alt1();
        let to_com = buf.g4_alt2() as u32;
        let from_obj = buf.ig2();
        let to_obj = buf.g2();
        IfButtonD {
            from_com,
            from_slot,
            from_obj,
            to_com,
            to_slot,
            to_obj,
        }
    }
}
