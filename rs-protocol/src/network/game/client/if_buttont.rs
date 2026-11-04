//! `IfButtonT` — rev-647 IF_BUTTONT (opcode 6, fixed 16).
//!
//! Write order in the rev-647 client (`ISAACRandom.java:26-33`).
//! The globals `anInt5794 / anInt123 / anInt2320` track the SOURCE menu
//! entry (set in `Js5TextureSource.java:22-28` when the drag begins);
//! `arg0` is the TARGET menu entry at send time:
//! ```text
//! p4(arg0.id)             // TARGET component hash
//! ip2(arg0.child_id)      // TARGET slot
//! ip2(anInt5794)          // SOURCE slot         (source.child_id)
//! p2_alt1(anInt123)       // SOURCE obj          (source.object_id)
//! ip4(anInt2320)          // SOURCE component hash (source.id)
//! p2(arg0.object_id)      // TARGET obj
//! ```
//! The RuneScript `[opheldu, <from>]` trigger is keyed on the SOURCE
//! (dragged) item, so we map `from_* = source_*, to_* = target_*`.
//! Server read below mirrors each write in the same order.

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(16), UserEvent)]
pub struct IfButtonT {
    pub from_com: u32,
    pub from_slot: u16,
    pub from_obj: u16,
    pub to_com: u32,
    pub to_slot: u16,
    pub to_obj: u16,
}

impl ClientProtMessage for IfButtonT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let to_com = buf.g4s() as u32;
        let to_slot = buf.ig2();
        let from_slot = buf.ig2();
        let from_obj = buf.g2_alt1();
        let from_com = buf.ig4s() as u32;
        let to_obj = buf.g2();
        IfButtonT {
            from_com,
            from_slot,
            from_obj,
            to_com,
            to_slot,
            to_obj,
        }
    }
}
