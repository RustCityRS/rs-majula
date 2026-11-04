use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetPosition, Buffered, Fixed)]
pub struct IfSetPosition {
    pub com: u16,
    pub x: u16,
    pub y: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetPosition {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.x);
        buf.p2(self.y);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.x) + size_of_val(&self.y)
    }
}

// `IfSetPosition` — move an interface component.
// Rev-647 opcode 37 (IF_SETPOSITION), fixed 8.
// Layout: `[ip4 component_hash][p2 x][p2 y]`.

#[cfg(since_647)]
#[server_prot(IfSetPosition, Buffered, Fixed)]
pub struct IfSetPosition {
    pub component_hash: i32,
    pub x: i16,
    pub y: i16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetPosition {
    // Client reads `g4_alt1 component_hash` then two `ig2b_alt1` coords
    // (`MapAreaConfig.java`, IF_SETPOSITION). `ip4`/`p2`/`p2` wrote the right
    // byte count in the wrong order. The client passes the reads to
    // `method1172(third, hash, second)`, which stores the second as
    // `anInt5264` and the third as `anInt5267`; state change 11 applies them
    // as x and y (`VarbitType`: `x = anInt5264`, `y = anInt5267`, both modes
    // 0, so the position is set outright, not offset).
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt1(self.component_hash);
        buf.ip2_alt1(self.x as u16);
        buf.ip2_alt1(self.y as u16);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash) + size_of_val(&self.x) + size_of_val(&self.y)
    }
}
