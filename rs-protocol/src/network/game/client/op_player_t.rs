//! `OpPlayerT` — rev-647 OPPLAYERT (opcode 15, fixed 11).
//!
//! 647 unifies "use item on player" and "cast spell on player" into a single
//! targeted packet, carrying the selected SOURCE menu entry alongside the
//! TARGET player.
//!
//! Client write order (`Statics1.java`, `MiniMenuAction.TGT_PLAYER`):
//! ```text
//! p2_alt1(entity)     // TARGET pid                                          -> pid
//! ip2(anInt5794)      // SOURCE slot   (source component.child_id)           -> from_slot
//! p4(anInt2320)       // SOURCE com    (source component.id, iface<<16|comp) -> from_com
//! p1_alt2(ctrl)       // ctrl-run flag                                       -> ctrl_run
//! ip2(anInt123)       // SOURCE obj id (source component.object_id)          -> from_obj
//! ```
//! When the source is an inventory item, `from_obj`/`from_slot` drive
//! `[opplayeru,...]`; when it is a spell, `from_com` drives `[opplayert,...]`.

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(11), UserEvent)]
pub struct OpPlayerT {
    /// Target player id (local pid).
    pub pid: u16,
    /// Source item id (the obj being "used"); -1/0 when the source is a spell.
    pub from_obj: u16,
    /// Source inventory slot of the used item.
    pub from_slot: u16,
    /// Source component hash (`interface_id << 16 | component_id`).
    pub from_com: u32,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpPlayerT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let pid = buf.g2_alt1();
        let from_slot = buf.ig2();
        let from_com = buf.g4s() as u32;
        let ctrl_run = buf.g1_alt2() != 0;
        let from_obj = buf.ig2();
        OpPlayerT {
            pid,
            from_obj,
            from_slot,
            from_com,
            ctrl_run,
        }
    }
}
