//! `SpotanimSpecific` — rev-647 SPOTANIM_SPECIFIC (opcode 126, fixed 12): a
//! spot animation on one tile, npc or player, sent to one player.
//!
//! Client read order (`MapAreaConfig`'s SPOTANIM_SPECIFIC branch):
//! `g1_alt2 packed; g1_alt3 channel; g4 target; g2 spotanim; ig2 height;
//! ig2_alt1 delay`. The target word says what it lands on: bit 30 set, a tile
//! (`level << 28 | x << 14 | z`); else bit 29, an npc (index in the low 16
//! bits); else bit 28, a player (likewise). Before this was named `MapAnim`,
//! which is opcode 109 at 647.

use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(SpotanimSpecific, Buffered, Fixed)]
pub struct SpotanimSpecific {
    /// Low 3 bits the rotation, bits 3-6 the label (`15` for none).
    pub packed: u8,
    /// Which of the entity's spotanim channels it plays on.
    pub channel: u8,
    /// What it plays on; see the module docs.
    pub target: i32,
    pub spotanim: u16,
    /// Subtracted from the ground height it is drawn at.
    pub height: u16,
    pub delay: u16,
}

impl ServerProtMessage for SpotanimSpecific {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.packed);
        buf.p1_alt3(self.channel);
        buf.p4(self.target);
        buf.p2(self.spotanim);
        buf.ip2(self.height);
        buf.ip2_alt1(self.delay);
    }

    fn sizeof(&self) -> usize {
        12
    }
}
