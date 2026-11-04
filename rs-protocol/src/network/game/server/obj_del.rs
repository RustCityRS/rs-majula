use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(ObjDel, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct ObjDel {
    pub coord: u8,
    pub id: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for ObjDel {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.id)
    }
}

// `ObjDel` — rev-647 OBJ_DEL (opcode 103, Fixed 3).
// Removes a ground item from a zone.
// Layout: `[p1_alt1 coord][ip2_alt1 obj_id]`.

#[cfg(since_647)]
use crate::network::game::server::server_game_zone_message::ServerGameZoneMessage;

#[cfg(since_647)]
#[server_prot(ObjDel, Buffered, Fixed)]
pub struct ObjDel {
    pub coord: u8,
    pub obj_id: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for ObjDel {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt1(self.coord);
        buf.ip2_alt1(self.obj_id);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.coord) + size_of_val(&self.obj_id)
    }
}

#[cfg(since_647)]
impl ServerGameZoneMessage for ObjDel {
    fn coord(&self) -> u32 {
        self.coord as u32
    }
}
