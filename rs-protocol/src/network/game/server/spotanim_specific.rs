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

#[cfg(since_647)]
#[server_prot(SpotanimSpecific, Buffered, Fixed)]
pub struct SpotanimSpecific {
    pub packed: u8,
    pub channel: u8,
    pub target: i32,
    pub spotanim: u16,
    pub height: u16,
    pub delay: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for SpotanimSpecific {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.packed);
        buf.p1_alt3(self.channel);
        buf.p4(self.target);
        buf.p2(self.spotanim);
        buf.ip2(self.height);
        buf.ip2_alt1(self.delay);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.packed)
            + size_of_val(&self.channel)
            + size_of_val(&self.target)
            + size_of_val(&self.spotanim)
            + size_of_val(&self.height)
            + size_of_val(&self.delay)
    }
}
