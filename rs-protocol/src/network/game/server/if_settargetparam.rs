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
use rs_ver::TypeId;

#[cfg(since_530)]
#[server_prot(IfSetTargetParam, Buffered, Fixed)]
pub struct IfSetTargetParam {
    pub com: i32,
    pub slot1: u16,
    pub slot2: u16,
    pub inv: TypeId,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for IfSetTargetParam {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt3(self.inv.g2());
        buf.p4_alt1(self.com);
        buf.p2_alt2(self.verify);
        buf.p2_alt1(self.slot2);
        buf.p2_alt2(self.slot1);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
            + size_of_val(&self.slot1)
            + size_of_val(&self.slot2)
            + size_of::<u16>()
            + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetTargetParam {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p2_alt1(self.slot1);
            buf.p2_alt3(self.inv.g2());
            buf.p2_alt2(self.slot2);
        }
        #[cfg(since_742)]
        {
            buf.p2_alt1(self.inv.g2());
            buf.p2_alt2(self.slot1);
            buf.p2_alt2(self.slot2);
        }
        buf.p4_alt3(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
            + size_of_val(&self.slot1)
            + size_of_val(&self.slot2)
            + size_of::<u16>()
    }
}
