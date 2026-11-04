#[cfg(any(before_245_2, since_647))]
use crate::network::game::server::ServerProtMessage;
#[cfg(any(before_245_2, since_647))]
use crate::network::game::server_prot::ServerProt;
#[cfg(any(before_245_2, since_647))]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(any(before_245_2, since_647))]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(any(before_245_2, since_647))]
use rs_io::{Packet, PacketFrame};
#[cfg(any(before_245_2, since_647))]
use rs_protocol_macros::server_prot;

#[cfg(before_245_2)]
#[server_prot(IfSetRecol, Buffered, Fixed)]
pub struct IfSetRecol {
    pub com: u16,
    pub src: u16,
    pub dst: u16,
}

#[cfg(before_245_2)]
impl ServerProtMessage for IfSetRecol {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.src);
        buf.p2(self.dst);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.src) + size_of_val(&self.dst)
    }
}

// `IfSetRecol` — recolour an interface component's model palette.
// Rev-647 opcode 132 (IF_SETRECOL), fixed 9.
// Layout: `[p1 slot][p2 src_colour][p2 dst_colour][p4 component_hash]`.
//
// **Field names are inferred**: 2004scape (rev-225) carries
// `(com, src, dst)` as 3× u16 = 6 bytes. Rev-647 is 9 bytes with an
// extra u8 + a u32 component hash. Without client-side method
// disassembly we can't be 100% sure which u16 is `src` vs `dst`;
// following the 2004scape arg order: first-on-wire = `src_colour`,
// second = `dst_colour`, `slot` = the extra byte (likely the palette
// slot index to replace, default 0).
//
// Client read order (from decompiled rev-647 `MapAreaConfig`):
// ```text
// int slot   = input_packet.g1();
// int field1 = input_packet.g2();     // first u16 on the wire
// int field2 = input_packet.g2();     // second u16 on the wire
// int hash   = input_packet.g4();
// ```

#[cfg(since_647)]
#[server_prot(IfSetRecol, Buffered, Fixed)]
pub struct IfSetRecol {
    pub component_hash: i32,
    pub src_colour: u16,
    pub dst_colour: u16,
    pub slot: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetRecol {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.slot);
        buf.p2(self.src_colour);
        buf.p2(self.dst_colour);
        buf.p4(self.component_hash);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash)
            + size_of_val(&self.src_colour)
            + size_of_val(&self.dst_colour)
            + size_of_val(&self.slot)
    }
}
