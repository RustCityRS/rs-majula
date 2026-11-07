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

#[cfg(since_647)]
#[server_prot(IfSetNpcHead, Buffered, Fixed)]
pub struct IfSetNpcHead {
    pub com: i32,
    pub npc: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetNpcHead {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.npc);
        buf.p4_alt2(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.npc)
    }
}
