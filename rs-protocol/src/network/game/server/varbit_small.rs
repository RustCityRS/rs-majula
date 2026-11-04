//! `VarbitSmall` — update a varbit with a value in `0..=255`.
//! Layout: `[p1_alt3 value][ip2 varbit]` — fixed 3 bytes, opcode 28.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(VarbitSmall, Buffered, Fixed)]
pub struct VarbitSmall {
    pub varbit: u16,
    pub value: i32,
}

impl ServerProtMessage for VarbitSmall {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt3(self.value as u8);
        buf.ip2(self.varbit);
    }

    fn sizeof(&self) -> usize {
        // u8 value byte + u16 varbit = 3
        3
    }
}
