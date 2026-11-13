#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;

#[cfg(since_647)]
#[server_prot(NpcAnimSpecific, Buffered, Fixed)]
pub struct NpcAnimSpecific {
    pub anims: [u16; 4],
    pub delay: u8,
    pub npc_uid: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for NpcAnimSpecific {
    fn encode(&self, buf: &mut Packet) {
        for a in &self.anims {
            buf.p2_alt1(*a);
        }
        buf.p1_alt1(self.delay);
        buf.p2_alt1(self.npc_uid);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.anims) + size_of_val(&self.delay) + size_of_val(&self.npc_uid)
    }
}
