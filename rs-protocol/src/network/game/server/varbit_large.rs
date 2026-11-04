//! `VarbitLarge` — update a varbit with a value outside `0..=255`.
//! Layout: `[ip2 varbit][ip4 value]` — fixed 6 bytes, opcode 114.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(VarbitLarge, Buffered, Fixed)]
pub struct VarbitLarge {
    pub varbit: u16,
    pub value: i32,
}

impl ServerProtMessage for VarbitLarge {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.varbit);
        buf.ip4(self.value);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.varbit) + size_of_val(&self.value)
    }
}
