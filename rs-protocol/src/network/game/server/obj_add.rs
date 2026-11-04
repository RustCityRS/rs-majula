use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(ObjAdd, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct ObjAdd {
    pub coord: u8,
    pub id: u16,
    pub count: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for ObjAdd {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.id);
        buf.p2(self.count);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.id) + size_of_val(&self.count)
    }
}

// `ObjAdd` — rev-647 OBJ_ADD (opcode 92, Fixed 5).
// Announces a ground item being spawned in a zone.
// Layout: `[ip2_alt1 count][p1 coord][ip2_alt1 obj_id]`.

#[cfg(since_647)]
use crate::network::game::server::server_game_zone_message::ServerGameZoneMessage;

#[cfg(since_647)]
#[server_prot(ObjAdd, Buffered, Fixed)]
pub struct ObjAdd {
    pub coord: u8,
    pub obj_id: u16,
    pub count: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for ObjAdd {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2_alt1(self.count);
        buf.p1(self.coord);
        buf.ip2_alt1(self.obj_id);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.count) + size_of_val(&self.coord) + size_of_val(&self.obj_id)
    }
}

#[cfg(since_647)]
impl ServerGameZoneMessage for ObjAdd {
    fn coord(&self) -> u32 {
        self.coord as u32
    }
}
