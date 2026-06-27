#[cfg(since_244)]
use crate::network::game::server::ServerProtMessage;
#[cfg(since_244)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_244)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_244)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_244)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_244)]
use rs_protocol_macros::server_prot;

#[cfg(since_244)]
#[server_prot(IfOpenOverlay, Buffered, Fixed)]
pub struct IfOpenOverlay {
    pub com: u16,
}

#[cfg(since_244)]
impl ServerProtMessage for IfOpenOverlay {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
    }
}
