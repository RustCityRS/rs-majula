use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(VarpLarge, Immediate, Fixed)]
pub struct VarpLarge {
    pub id: u16,
    pub val: i32,
}

#[cfg(before_647)]
impl ServerProtMessage for VarpLarge {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.id);
        buf.p4(self.val);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}

// `VarpLarge` — update a player var (varp) with a value outside `-128..=127`.
// Layout: `[p2 varp][p4_alt2 value]` — fixed 6 bytes, opcode 49.

#[cfg(since_647)]
#[server_prot(VarpLarge, Buffered, Fixed)]
pub struct VarpLarge {
    pub varp: u16,
    pub value: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for VarpLarge {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.varp);
        buf.p4_alt2(self.value);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.varp) + size_of_val(&self.value)
    }
}
