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
#[server_prot(CamSmoothreset, Buffered, Fixed)]
pub struct CamSmoothreset;

#[cfg(since_647)]
impl ServerProtMessage for CamSmoothreset {
    fn encode(&self, _buf: &mut Packet) {}

    fn sizeof(&self) -> usize {
        0
    }
}
