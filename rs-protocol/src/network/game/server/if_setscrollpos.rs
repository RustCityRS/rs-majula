#[cfg(since_245_2)]
use crate::network::game::server::ServerProtMessage;
#[cfg(since_245_2)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_245_2)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_245_2)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_245_2)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_245_2)]
use rs_protocol_macros::server_prot;

#[cfg(all(since_245_2, before_647))]
#[server_prot(IfSetScrollPos, Buffered, Fixed)]
pub struct IfSetScrollPos {
    pub com: u16,
    pub y: u16,
}

#[cfg(all(since_245_2, before_647))]
impl ServerProtMessage for IfSetScrollPos {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.y);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.y)
    }
}

// `IfSetScrollPos` — move the scroll position of a scrollable interface
// component. Rev-647 opcode 58 (IF_SETSCROLLPOS), fixed 6.
// Layout: `[p4_alt2 component_hash][p2_alt1 scroll_pos]`.
//
// Client read order (from decompiled rev-647 `MapAreaConfig`):
// ```text
// int hash   = input_packet.g4_alt2();
// int scroll = input_packet.g2_alt1();
// ```

#[cfg(since_647)]
#[server_prot(IfSetScrollPos, Buffered, Fixed)]
pub struct IfSetScrollPos {
    pub component_hash: i32,
    pub scroll_pos: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetScrollPos {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.component_hash);
        buf.p2_alt1(self.scroll_pos);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash) + size_of_val(&self.scroll_pos)
    }
}
