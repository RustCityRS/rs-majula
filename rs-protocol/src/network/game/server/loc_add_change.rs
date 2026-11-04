use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(LocAddChange, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct LocAddChange {
    pub coord: u8,
    pub shape_angle: u8,
    pub id: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for LocAddChange {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p1(self.shape_angle);
        buf.p2(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle) + size_of_val(&self.id)
    }
}

// `LocAddChange` — rev-647 LOC_ADD_CHANGE (opcode 44, Fixed 4).
// Announces a loc being spawned or replacing an existing one.
// Layout: `[p1_alt2 coord][p1_alt1 shape_rot][p2_alt1 loc_id]`.

#[cfg(since_647)]
use crate::network::game::server::server_game_zone_message::ServerGameZoneMessage;

#[cfg(since_647)]
#[server_prot(LocAddChange, Buffered, Fixed)]
pub struct LocAddChange {
    pub coord: u8,
    pub shape_rot: u8,
    pub id: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for LocAddChange {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.coord);
        buf.p1_alt1(self.shape_rot);
        buf.p2_alt1(self.id);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.coord) + size_of_val(&self.shape_rot) + size_of_val(&self.id)
    }
}

#[cfg(since_647)]
impl ServerGameZoneMessage for LocAddChange {
    fn coord(&self) -> u32 {
        self.coord as u32
    }
}
