//! `NpcAnimSpecific` — play an animation on a specific NPC by uid.
//! Rev-647 opcode 60, fixed 11.
//!
//! Client read order (`MapAreaConfig.java`):
//! `for i in 0..4 { ig2 anims[i] }; g1_alt3 delay; ig2 npc_uid`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(NpcAnimSpecific, Buffered, Fixed)]
pub struct NpcAnimSpecific {
    /// Four chained animation ids (each 65535 = `-1`).
    /// Mirrors the rev-647 client's `int[] var301 = new int[4]`.
    pub anims: [u16; 4],
    /// Animation delay byte.
    pub delay: u8,
    /// Target NPC server-uid (entity-table key).
    pub npc_uid: u16,
}

impl ServerProtMessage for NpcAnimSpecific {
    fn encode(&self, buf: &mut Packet) {
        for a in &self.anims {
            buf.ip2(*a);
        }
        buf.p1_alt3(self.delay);
        buf.ip2(self.npc_uid);
    }

    fn sizeof(&self) -> usize {
        // 4 × u16 anims + u8 delay + u16 npc_uid = 11
        11
    }
}
