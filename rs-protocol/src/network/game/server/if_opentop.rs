//! `IfOpentop` — open a top-level interface.
//! Rev-647 opcode 61 (IF_OPENTOP), fixed 3.
//! Layout: `[ip2 interface_id][p1_alt3 type=0]`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(IfOpentop, Buffered, Fixed)]
pub struct IfOpentop {
    pub interface_id: u16,
}

impl ServerProtMessage for IfOpentop {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.interface_id);
        buf.p1_alt3(0);
    }

    fn sizeof(&self) -> usize {
        3
    }
}
