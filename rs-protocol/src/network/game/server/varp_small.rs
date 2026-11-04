use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(VarpSmall, Immediate, Fixed)]
pub struct VarpSmall {
    pub id: u16,
    pub val: u8,
}

#[cfg(before_647)]
impl ServerProtMessage for VarpSmall {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.id);
        buf.p1(self.val);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}

// `VarpSmall` — update a player var (varp) with a value in `-128..=127`.
//
// Mirrors 2004scape's `model/VarpSmall.ts`. Wire opcode = rev-647 `116`.
// Layout: `[p1_alt2 value][p2 varp]` — fixed 3 bytes.

#[cfg(since_647)]
#[server_prot(VarpSmall, Buffered, Fixed)]
pub struct VarpSmall {
    pub varp: u16,
    pub value: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for VarpSmall {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.value as u8);
        buf.p2(self.varp);
    }

    fn sizeof(&self) -> usize {
        // u8 value byte + u16 varp = 3
        3
    }
}
