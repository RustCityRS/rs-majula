use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

/// Stops every voice-over clip the client is playing or has queued.
#[server_prot(VorbisSpeechStop, Buffered, Fixed)]
pub struct VorbisSpeechStop;

impl ServerProtMessage for VorbisSpeechStop {
    fn encode(&self, _: &mut Packet) {}

    fn sizeof(&self) -> usize {
        0
    }
}
