//! `IfSetRetex` — texture replacement on an interface component model.
//! Rev-647 opcode 63 (IF_SETRETEX), fixed 9. Parallel to IF_SETRECOL.
//!
//! Client read order (`MapAreaConfig.java`):
//! `g4 component_hash; g1_alt2 index; g2_alt1 src_tex; g2_alt1 dst_tex`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(IfSetRetex, Buffered, Fixed)]
pub struct IfSetRetex {
    /// Packed `(interface_id << 16) | component_id`.
    pub component_hash: i32,
    /// Index into the component's texture slot table.
    pub index: u8,
    /// Source texture id.
    pub src_tex: u16,
    /// Destination/replacement texture id.
    pub dst_tex: u16,
}

impl ServerProtMessage for IfSetRetex {
    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.component_hash);
        buf.p1_alt2(self.index);
        buf.p2_alt1(self.src_tex);
        buf.p2_alt1(self.dst_tex);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash)
            + size_of_val(&self.index)
            + size_of_val(&self.src_tex)
            + size_of_val(&self.dst_tex)
    }
}
