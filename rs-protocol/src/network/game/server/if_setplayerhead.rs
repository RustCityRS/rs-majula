use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetPlayerHead, Buffered, Fixed)]
pub struct IfSetPlayerHead {
    pub com: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetPlayerHead {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
    }
}

// `IfSetPlayerHead` — show the local player's head model on an interface.
// Rev-647 opcode 25 (IF_SETPLAYERHEAD), fixed 4.
// Layout: `[ip4 component_hash]`.

#[cfg(since_647)]
#[server_prot(IfSetPlayerHead, Buffered, Fixed)]
pub struct IfSetPlayerHead {
    pub component_hash: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetPlayerHead {
    fn encode(&self, buf: &mut Packet) {
        buf.ip4(self.component_hash);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash)
    }
}
