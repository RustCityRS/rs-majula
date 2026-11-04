use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(LocDel, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct LocDel {
    pub coord: u8,
    pub shape_angle: u8,
}

#[cfg(before_647)]
impl ServerProtMessage for LocDel {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p1(self.shape_angle);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.shape_angle)
    }
}

// `LocDel` — rev-647 LOC_DEL (opcode 17, Fixed 2).
// Removes a loc from a zone.
// Layout: `[p1 shape_rot][p1_alt2 coord]`.

#[cfg(since_647)]
use crate::network::game::server::server_game_zone_message::ServerGameZoneMessage;

#[cfg(since_647)]
#[server_prot(LocDel, Buffered, Fixed)]
pub struct LocDel {
    pub coord: u8,
    pub shape_rot: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for LocDel {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.shape_rot);
        buf.p1_alt2(self.coord);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.shape_rot) + size_of_val(&self.coord)
    }
}

#[cfg(since_647)]
impl ServerGameZoneMessage for LocDel {
    fn coord(&self) -> u32 {
        self.coord as u32
    }
}
