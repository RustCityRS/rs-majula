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

#[cfg(since_647)]
#[server_prot(IfSetPosition, Buffered, Fixed)]
pub struct IfSetPosition {
    pub com: i32,
    pub x: u16,
    pub y: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetPosition {
    // Client reads `g4s_alt3 component_hash` then two `ig2b_alt1` coords
    // (`MapAreaConfig.java`, IF_SETPOSITION). `p4_alt1`/`p2`/`p2` wrote the right
    // byte count in the wrong order. The client passes the reads to
    // `method1172(third, hash, second)`, which stores the second as
    // `anInt5264` and the third as `anInt5267`; state change 11 applies them
    // as x and y (`VarbitType`: `x = anInt5264`, `y = anInt5267`, both modes
    // 0, so the position is set outright, not offset).
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt3(self.com);
        buf.p2_alt3(self.x);
        buf.p2_alt3(self.y);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.x) + size_of_val(&self.y)
    }
}
