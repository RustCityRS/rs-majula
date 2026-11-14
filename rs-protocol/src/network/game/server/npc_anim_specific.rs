#[cfg(since_530)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_530)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_530)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_530)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_530)]
use rs_protocol_macros::server_prot;

#[cfg(all(since_530, before_647))]
#[server_prot(NpcAnimSpecific, Buffered, Fixed)]
pub struct NpcAnimSpecific {
    pub npc_uid: u16,
    pub delay: u8,
    pub seq: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for NpcAnimSpecific {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.npc_uid);
        buf.p1_alt3(self.delay);
        buf.p2(self.seq);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.npc_uid) + size_of_val(&self.delay) + size_of_val(&self.seq)
    }
}

#[cfg(since_647)]
#[server_prot(NpcAnimSpecific, Buffered, Fixed)]
pub struct NpcAnimSpecific {
    pub anims: [i32; 4],
    pub delay: u8,
    pub npc_uid: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for NpcAnimSpecific {
    #[cfg(before_742)]
    fn encode(&self, buf: &mut Packet) {
        for a in &self.anims {
            buf.p2_alt1(*a as u16);
        }
        buf.p1_alt1(self.delay);
        buf.p2_alt1(self.npc_uid);
    }

    #[cfg(since_742)]
    fn encode(&self, buf: &mut Packet) {
        for a in &self.anims {
            buf.p4_alt2(*a);
        }
        buf.p1_alt2(self.delay);
        buf.p2(self.npc_uid);
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let anims = size_of::<[u16; 4]>();
        #[cfg(since_742)]
        let anims = size_of_val(&self.anims);
        anims + size_of_val(&self.delay) + size_of_val(&self.npc_uid)
    }
}
