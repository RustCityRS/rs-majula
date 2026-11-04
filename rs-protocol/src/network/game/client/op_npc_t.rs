//! `OpNpcT` — rev-647 OPNPCT (opcode 66, fixed 11).
//!
//! 647 unifies "use item on NPC" and "cast spell on NPC" into a single
//! targeted packet. The selected SOURCE menu entry (set when the player picks
//! a spell / selects an item to use, in `Js5TextureSource.set_target`) is
//! carried alongside the TARGET npc.
//!
//! Client write order (`Statics1.java`, `MiniMenuAction.TGT_NPC`):
//! ```text
//! p2_alt1(anInt123)   // SOURCE obj id (source component.object_id)          -> from_obj
//! p2_alt1(anInt5794)  // SOURCE slot   (source component.child_id)           -> from_slot
//! ip2(entity)         // TARGET nid                                          -> nid
//! p4_alt1(anInt2320)  // SOURCE com    (source component.id, iface<<16|comp) -> from_com
//! p1_alt1(ctrl)       // ctrl-run flag                                       -> ctrl_run
//! ```
//! When the source is an inventory item, `from_obj` is that item id and
//! `from_slot` is its inventory slot (drives `last_useitem`/`last_useslot` and
//! the `[opnpcu,...]` trigger). When the source is a spell, `from_obj` is -1/0
//! and `from_com` identifies the spell component (drives `[opnpct,...]`).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(11), UserEvent)]
pub struct OpNpcT {
    /// Target npc id (local nid).
    pub nid: u16,
    /// Source item id (the obj being "used"); -1/0 when the source is a spell.
    pub from_obj: u16,
    /// Source inventory slot of the used item.
    pub from_slot: u16,
    /// Source component hash (`interface_id << 16 | component_id`) — the spell
    /// or inventory component selected as the use/cast source.
    pub from_com: u32,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpNpcT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let from_obj = buf.g2_alt1();
        let from_slot = buf.g2_alt1();
        let nid = buf.ig2();
        let from_com = buf.g4_alt1() as u32;
        let ctrl_run = buf.g1_alt1() != 0;
        OpNpcT {
            nid,
            from_obj,
            from_slot,
            from_com,
            ctrl_run,
        }
    }
}
