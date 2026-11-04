use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(ObjCount, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct ObjCount {
    pub coord: u8,
    pub id: u16,
    pub old_count: u16,
    pub new_count: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for ObjCount {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.id);
        buf.p2(self.old_count);
        buf.p2(self.new_count);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.id)
            + size_of_val(&self.old_count)
            + size_of_val(&self.new_count)
    }
}

// `ObjCount` — rev-647 OBJ_COUNT (opcode 121, Fixed 7).
// Updates the stack size of an existing ground item in a zone.
// Layout: `[p1 coord][p2 obj_id][p2 old_count][p2 new_count]`.

#[cfg(since_647)]
use crate::network::game::server::server_game_zone_message::ServerGameZoneMessage;

#[cfg(since_647)]
#[server_prot(ObjCount, Buffered, Fixed)]
pub struct ObjCount {
    pub coord: u8,
    pub obj_id: u16,
    pub old_count: u16,
    pub new_count: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for ObjCount {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.obj_id);
        buf.p2(self.old_count);
        buf.p2(self.new_count);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.coord)
            + size_of_val(&self.obj_id)
            + size_of_val(&self.old_count)
            + size_of_val(&self.new_count)
    }
}

#[cfg(since_647)]
impl ServerGameZoneMessage for ObjCount {
    fn coord(&self) -> u32 {
        self.coord as u32
    }
}
