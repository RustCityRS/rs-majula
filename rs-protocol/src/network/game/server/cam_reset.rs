use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(any(before_530, since_647))]
#[server_prot(CamReset, Buffered, Fixed)]
pub struct CamReset;

#[cfg(any(before_530, since_647))]
impl ServerProtMessage for CamReset {
    fn encode(&self, _: &mut Packet) {}

    fn sizeof(&self) -> usize {
        0
    }
}

#[cfg(all(since_530, before_647))]
#[server_prot(CamReset, Buffered, Fixed)]
pub struct CamReset {
    pub verify: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for CamReset {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.verify);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.verify)
    }
}
