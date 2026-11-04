use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetNpcHead, Buffered, Fixed)]
pub struct IfSetNpcHead {
    pub com: u16,
    pub npc: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetNpcHead {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.npc);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.npc)
    }
}

// `IfSetNpcHead` — show an NPC's head model on an interface.
// Rev-647 opcode 129 (IF_SETNPCHEAD), fixed 6.
// Layout: `[ip2 npc_id][p4_alt2 component_hash]`.

#[cfg(since_647)]
#[server_prot(IfSetNpcHead, Buffered, Fixed)]
pub struct IfSetNpcHead {
    pub component_hash: i32,
    pub npc_id: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetNpcHead {
    fn encode(&self, buf: &mut Packet) {
        buf.ip2(self.npc_id);
        buf.p4_alt2(self.component_hash);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.component_hash) + size_of_val(&self.npc_id)
    }
}
