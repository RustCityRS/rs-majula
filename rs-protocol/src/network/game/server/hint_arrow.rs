use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(HintArrow, Buffered, Fixed)] // TODO: what should priority be?
pub struct HintArrow {
    pub hint: u8,
    pub arg1: u16,
    pub arg2: u16,
    pub arg3: u8,
}

#[cfg(before_647)]
impl ServerProtMessage for HintArrow {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.hint);
        buf.p2(self.arg1);
        buf.p2(self.arg2);
        buf.p1(self.arg3);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.hint)
            + size_of_val(&self.arg1)
            + size_of_val(&self.arg2)
            + size_of_val(&self.arg3)
    }
}

// `HintArrow` — draw/hide a hint arrow (the red floating arrow for quests etc.).
// Rev-647 opcode 45 (HINT_ARROW), fixed 12.
//
// 2004scape models the three targeting variants (coord / npc / stop) as
// a single class with branch-encoded fields. We mirror that with a
// `HintArrowKind` enum inside `HintArrow`.
//
// Layout by variant (all pad to 12 bytes):
//
// - **Coord**: `[p1 type=2][p1 headicon=0][p1 level][p2 x][p2 z][p1 height][p2 unused=0][p2 model_id]`
// - **Npc**:   `[p1 type=1][p1 headicon=0][p2 npc_index][p1*6 padding][p2 model_id]`
// - **Stop**:  `[p1 type=0][p1*11 padding]`

#[cfg(since_647)]
pub enum HintArrowKind {
    /// Point at a world coordinate.
    Coord {
        x: u16,
        z: u16,
        level: u8,
        height: u8,
    },
    /// Point at an NPC by viewport index.
    Npc { npc_index: u16 },
    /// Point at a player by slot index (rev-647 type-tag 10).
    Player { player_index: u16 },
    /// Remove any current hint arrow.
    Stop,
}

#[cfg(since_647)]
#[server_prot(HintArrow, Buffered, Fixed)]
pub struct HintArrow {
    pub kind: HintArrowKind,
    /// Arrow model ID (typically `0xFFFF` for default).
    pub model_id: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for HintArrow {
    fn encode(&self, buf: &mut Packet) {
        match self.kind {
            HintArrowKind::Coord {
                x,
                z,
                level,
                height,
            } => {
                buf.p1(2);
                buf.p1(0);
                buf.p1(level);
                buf.p2(x);
                buf.p2(z);
                buf.p1(height);
                buf.p2(0);
                buf.p2(self.model_id);
            }
            HintArrowKind::Npc { npc_index } => {
                buf.p1(1);
                buf.p1(0);
                buf.p2(npc_index);
                for _ in 0..6 {
                    buf.p1(0);
                }
                buf.p2(self.model_id);
            }
            HintArrowKind::Player { player_index } => {
                // Same shape as Npc but type-tag 10 (rev-647 client
                // switch on type: 2=coord, 1=npc, 10=player, 0=stop).
                buf.p1(10);
                buf.p1(0);
                buf.p2(player_index);
                for _ in 0..6 {
                    buf.p1(0);
                }
                buf.p2(self.model_id);
            }
            HintArrowKind::Stop => {
                buf.p1(0);
                for _ in 0..11 {
                    buf.p1(0);
                }
            }
        }
    }

    fn sizeof(&self) -> usize {
        12
    }
}
