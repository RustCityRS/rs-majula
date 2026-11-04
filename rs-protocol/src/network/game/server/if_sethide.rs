use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetHide, Buffered, Fixed)]
pub struct IfSetHide {
    pub com: u16,
    pub hide: bool,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetHide {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p1(self.hide as u8);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.hide)
    }
}

// `IfSetHide` — toggle visibility of an interface component.
// Rev-647 opcode 105 (IF_SETHIDE), fixed 5.
//
// Client read order (`MapAreaConfig.java`):
// `g4_alt1 component_hash; g1_alt1 hidden`.

#[cfg(since_647)]
#[server_prot(IfSetHide, Buffered, Fixed)]
pub struct IfSetHide {
    pub component_hash: i32,
    pub hidden: bool,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetHide {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt1(self.component_hash);
        buf.p1_alt1(self.hidden as u8);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash) + 1
    }
}
