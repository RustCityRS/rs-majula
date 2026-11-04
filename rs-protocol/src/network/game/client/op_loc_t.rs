//! `OpLocT` — rev-647 OPLOCT (opcode 46, fixed 15).
//!
//! 647 unifies "use item on loc" and "cast spell on loc" into a single
//! targeted packet, carrying the selected SOURCE menu entry alongside the
//! TARGET loc.
//!
//! Client write order (`Statics1.java`, `MiniMenuAction.TGT_LOC`):
//! ```text
//! p2_alt1(anInt123)        // SOURCE obj id                                  -> from_obj
//! p2_alt1(anInt5794)       // SOURCE slot                                    -> from_slot
//! p2(region_z + arg2)      // TARGET tile z                                  -> z
//! p4_alt2(anInt2320)       // SOURCE com    (iface<<16|comp)                 -> from_com
//! p2_alt1(region_x + arg1) // TARGET tile x                                  -> x
//! p1_alt3(ctrl)            // ctrl-run flag                                  -> ctrl_run
//! ip2(entity_long >>> 32)  // TARGET loc id                                  -> loc_id
//! ```
//! When the source is an inventory item, `from_obj`/`from_slot` drive
//! `[oplocu,...]`; when it is a spell, `from_com` drives `[oploct,...]`.

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(15), UserEvent)]
pub struct OpLocT {
    /// Target loc id.
    pub loc_id: u16,
    /// Target tile x.
    pub x: u16,
    /// Target tile z.
    pub z: u16,
    /// Source item id (the obj being "used"); -1/0 when the source is a spell.
    pub from_obj: u16,
    /// Source inventory slot of the used item.
    pub from_slot: u16,
    /// Source component hash (`interface_id << 16 | component_id`).
    pub from_com: u32,
    pub ctrl_run: bool,
}

impl ClientProtMessage for OpLocT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let from_obj = buf.g2_alt1();
        let from_slot = buf.g2_alt1();
        let z = buf.g2();
        let from_com = buf.g4_alt2() as u32;
        let x = buf.g2_alt1();
        let ctrl_run = buf.g1_alt3() != 0;
        let loc_id = buf.ig2();
        OpLocT {
            loc_id,
            x,
            z,
            from_obj,
            from_slot,
            from_com,
            ctrl_run,
        }
    }
}
