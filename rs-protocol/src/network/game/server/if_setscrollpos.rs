#[cfg(since_245_2)]
use crate::network::game::server::ServerProtMessage;
#[cfg(since_245_2)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_245_2)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_245_2)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_245_2)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_245_2)]
use rs_protocol_macros::server_prot;

#[cfg(all(since_245_2, before_530))]
#[server_prot(IfSetScrollPos, Buffered, Fixed)]
pub struct IfSetScrollPos {
    pub com: u16,
    pub y: u16,
}

#[cfg(all(since_245_2, before_530))]
impl ServerProtMessage for IfSetScrollPos {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.y);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.y)
    }
}

#[cfg(since_530)]
#[server_prot(IfSetScrollPos, Buffered, Fixed)]
pub struct IfSetScrollPos {
    pub com: i32,
    pub y: u16,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for IfSetScrollPos {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt3(self.com);
        buf.p2_alt1(self.y);
        buf.p2(self.verify);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.y) + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetScrollPos {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        buf.p4_alt2(self.com);
        buf.p2_alt2(self.y);
        #[cfg(since_742)]
        buf.p4_alt3(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.y)
    }
}
