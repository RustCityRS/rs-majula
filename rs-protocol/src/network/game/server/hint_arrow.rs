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

// `HintArrow` — draw/hide a hint arrow (the floating arrow for quests etc.).
// Rev-647 opcode 45 (HINT_ARROW), fixed 12, read by MapAreaConfig.java:1219.
//
// The client keeps eight arrows, one per slot. The first byte is
// `slot << 5 | type`: type 0 clears the slot; 1 points at an npc, 10 at a
// player; 2-6 at a tile (2 its centre, 3 the middle of its west edge, 4 east,
// 5 south, 6 north). The second is the icon, an index into the
// hint_headicons, hint_mapmarkers and hint_mapedge sheets; the client reads
// nothing after it when it is out of range.
//
// Layout by variant (all pad to 12 bytes):
//
// - **Coord**: `[p1 slot<<5|2..6][p1 icon][p1 level][p2 x][p2 z][p1 height][p2 range][p2 model_id]`
// - **Npc**:   `[p1 slot<<5|1][p1 icon][p2 npc_index][p1*6 padding][p2 model_id]`
// - **Player**: `[p1 slot<<5|10][p1 icon][p2 player_index][p1*6 padding][p2 model_id]`
// - **Stop**:  `[p1 slot<<5][p1*11 padding]`
//
// A coord arrow's height is in quarter units (the client shifts it left 2)
// and its range in 32-tile units: the client draws it on the minimap only
// while the player is within `range * 32` tiles of it (Linkable_Sub13).

#[cfg(since_647)]
pub enum HintArrowKind {
    /// Point at a world coordinate.
    Coord {
        offset: u8,
        x: u16,
        z: u16,
        level: u8,
        height: u8,
        range: u16,
    },
    /// Point at an NPC by viewport index.
    Npc { npc_index: u16 },
    /// Point at a player by slot index (rev-647 type-tag 10).
    Player { player_index: u16 },
    /// Remove the slot's arrow.
    Stop,
}

#[cfg(since_647)]
#[server_prot(HintArrow, Buffered, Fixed)]
pub struct HintArrow {
    pub slot: u8,
    pub icon: u8,
    pub kind: HintArrowKind,
    /// Arrow model ID (typically `0xFFFF` for default).
    pub model_id: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for HintArrow {
    fn encode(&self, buf: &mut Packet) {
        let head = |kind: u8| (self.slot << 5) | kind;
        match self.kind {
            HintArrowKind::Coord {
                offset,
                x,
                z,
                level,
                height,
                range,
            } => {
                buf.p1(head(offset));
                buf.p1(self.icon);
                buf.p1(level);
                buf.p2(x);
                buf.p2(z);
                buf.p1(height);
                buf.p2(range);
                buf.p2(self.model_id);
            }
            HintArrowKind::Npc { npc_index } => {
                buf.p1(head(1));
                buf.p1(self.icon);
                buf.p2(npc_index);
                for _ in 0..6 {
                    buf.p1(0);
                }
                buf.p2(self.model_id);
            }
            HintArrowKind::Player { player_index } => {
                buf.p1(head(10));
                buf.p1(self.icon);
                buf.p2(player_index);
                for _ in 0..6 {
                    buf.p1(0);
                }
                buf.p2(self.model_id);
            }
            HintArrowKind::Stop => {
                buf.p1(head(0));
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
