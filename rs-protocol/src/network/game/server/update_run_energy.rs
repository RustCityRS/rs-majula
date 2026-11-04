//! `UpdateRunEnergy` — update the player's displayed run-energy orb (0-100).
//! Rev-647 opcode 69 (UPDATE_RUNENERGY), fixed 1.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(UpdateRunEnergy, Buffered, Fixed)]
pub struct UpdateRunEnergy {
    pub energy: u8,
}

impl ServerProtMessage for UpdateRunEnergy {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.energy);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.energy)
    }
}
