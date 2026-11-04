use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetAnim, Buffered, Fixed)]
pub struct IfSetAnim {
    pub com: u16,
    pub seq: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.seq);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.seq)
    }
}

// `IfSetAnim` — play a sequence animation on an interface component.
// Rev-647 opcode 118 (IF_SETANIM), fixed 6.
// Layout: `[ip4 component_hash][ip2 seq_id]`.

#[cfg(since_647)]
#[server_prot(IfSetAnim, Buffered, Fixed)]
pub struct IfSetAnim {
    pub component_hash: i32,
    pub seq_id: i16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.ip4(self.component_hash);
        buf.ip2(self.seq_id as u16);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash) + size_of_val(&self.seq_id)
    }
}
