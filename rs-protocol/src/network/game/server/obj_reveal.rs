use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(ObjReveal, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct ObjReveal {
    pub coord: u8,
    pub id: u16,
    pub count: u16,
    pub receiver: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for ObjReveal {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.id);
        buf.p2(self.count);
        buf.p2(self.receiver);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.id)
            + size_of_val(&self.count)
            + size_of_val(&self.receiver)
    }
}

// `ObjReveal` — rev-647 OBJ_REVEAL (opcode 80, Fixed 7).
// Reveals a ground item to a specific player in a zone.
// Layout: `[ip2_alt1 obj_id][p2 receiver][p1_alt1 coord][ip2_alt1 count]`.

#[cfg(since_647)]
use crate::network::game::server::server_game_zone_message::ServerGameZoneMessage;

#[cfg(since_647)]
#[server_prot(ObjReveal, Buffered, Fixed)]
pub struct ObjReveal {
    pub coord: u8,
    pub obj_id: u16,
    pub count: u16,
    pub receiver: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for ObjReveal {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2_alt1(self.obj_id);
        buf.p2(self.receiver);
        buf.p1_alt1(self.coord);
        buf.ip2_alt1(self.count);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.count)
            + size_of_val(&self.receiver)
            + size_of_val(&self.coord)
            + size_of_val(&self.obj_id)
    }
}

#[cfg(since_647)]
impl ServerGameZoneMessage for ObjReveal {
    fn coord(&self) -> u32 {
        self.coord as u32
    }
}
