#[cfg(since_647)]
use crate::network::game::server::ServerProtMessage;
#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;
use rs_ver::TypeId;

#[cfg(since_647)]
#[server_prot(SpotanimSpecific, Buffered, Fixed)]
pub struct SpotanimSpecific {
    pub packed: u8,
    pub channel: u8,
    pub target: i32,
    pub spotanim: TypeId,
    pub height: u16,
    pub delay: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for SpotanimSpecific {
    #[cfg(before_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt3(self.packed);
        buf.p1_alt1(self.channel);
        buf.p4(self.target);
        buf.p2(self.spotanim.g2());
        buf.p2_alt1(self.height);
        buf.p2_alt3(self.delay);
    }

    #[cfg(since_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.target);
        buf.p2(self.spotanim.g2());
        buf.p1_alt2(self.channel);
        buf.p1_alt2(self.packed);
        buf.p2(self.height);
        buf.p2_alt2(self.delay);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.packed)
            + size_of_val(&self.channel)
            + size_of_val(&self.target)
            + size_of::<u16>()
            + size_of_val(&self.height)
            + size_of_val(&self.delay)
    }
}
