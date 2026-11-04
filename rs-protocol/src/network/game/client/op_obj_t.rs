//! `OpObjT` — rev-647 OPOBJT (opcode 47, fixed 15).
//!
//! 647 unifies "use item on ground obj" and "cast spell on ground obj" into a
//! single targeted packet. The selected SOURCE menu entry (set when the player
//! picks a spell / selects an item to use, in `Js5TextureSource.set_target`)
//! is carried alongside the TARGET ground obj.
//!
//! Client write order (`Statics1.java`, `MiniMenuAction.TGT_OBJ`):
//! ```text
//! ip2(anInt123)      // SOURCE obj   (source component.object_id)  -> from_obj
//! ip2(entity)        // TARGET obj id                              -> obj_id
//! p2(anInt5794)      // SOURCE slot  (source component.child_id)   -> from_slot
//! p4_alt2(anInt2320) // SOURCE com   (source component.id, iface<<16|comp) -> from_com
//! p1_alt3(ctrl)      // ctrl-run flag                              -> ctrl_run
//! p2_alt1(z)         // target tile z                              -> z
//! p2_alt1(x)         // target tile x                              -> x
//! ```
//! When the source is an inventory item, `from_obj` is that item id and
//! `from_slot` is its inventory slot (this drives `last_useitem`/`last_useslot`
//! and the `[opobju,...]` trigger). When the source is a spell, `from_obj` is
//! -1/0 and `from_com` identifies the spell component (drives `[opobjt,...]`).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(15), UserEvent)]
pub struct OpObjT {
    /// Target ground obj id.
    pub obj_id: u16,
    /// Target tile x.
    pub x: u16,
    /// Target tile z.
    pub z: u16,
    /// Source item id (the obj being "used"); -1/0 when the source is a spell.
    pub from_obj: u16,
    /// Source inventory slot of the used item.
    pub from_slot: u16,
    /// Source component hash (`interface_id << 16 | component_id`) — the spell
    /// or inventory component selected as the use/cast source.
    pub from_com: u32,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpObjT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let from_obj = buf.ig2();
        let obj_id = buf.ig2();
        let from_slot = buf.g2();
        let from_com = buf.g4_alt2() as u32;
        let ctrl_run = buf.g1_alt3() != 0;
        let z = buf.g2_alt1();
        let x = buf.g2_alt1();
        OpObjT {
            obj_id,
            x,
            z,
            from_obj,
            from_slot,
            from_com,
            ctrl_run,
        }
    }
}
