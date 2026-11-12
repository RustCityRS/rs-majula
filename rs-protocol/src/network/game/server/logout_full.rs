use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

/// Logs the client out to the login screen, whichever way it came in.
#[server_prot(LogoutFull, Immediate, Fixed)]
pub struct LogoutFull;

impl ServerProtMessage for LogoutFull {
    fn encode(&self, _: &mut Packet) {}

    fn sizeof(&self) -> usize {
        0
    }
}
