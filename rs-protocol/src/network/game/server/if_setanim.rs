use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[cfg(before_530)]
#[server_prot(IfSetAnim, Buffered, Fixed)]
pub struct IfSetAnim {
    pub com: u16,
    pub seq: TypeId,
}

#[cfg(before_530)]
impl ServerProtMessage for IfSetAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.seq.g2());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of::<u16>()
    }
}

#[cfg(since_530)]
#[server_prot(IfSetAnim, Buffered, Fixed)]
pub struct IfSetAnim {
    pub com: i32,
    pub seq: TypeId,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for IfSetAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt3(self.com);
        buf.p2_alt1(self.seq.g2());
        buf.p2_alt2(self.verify);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of::<u16>() + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetAnim {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt1(self.com);
        #[cfg(before_742)]
        buf.p2_alt1(self.seq.g2());
        #[cfg(since_742)]
        buf.p4(self.seq.int());
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let seq = size_of::<u16>();
        #[cfg(since_742)]
        let seq = size_of::<i32>();
        size_of_val(&self.com) + seq
    }
}
