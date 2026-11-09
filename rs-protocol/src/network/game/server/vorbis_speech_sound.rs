use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

/// Plays a voice-over clip on the voice channel: the client plays it only while the
/// player's voice-over volume is above zero, and 65535 is no clip.
#[server_prot(VorbisSpeechSound, Buffered, Fixed)]
pub struct VorbisSpeechSound {
    pub vorbis: u16,
    pub loops: u8,
    pub delay: u16,
    pub volume: u8,
}

impl ServerProtMessage for VorbisSpeechSound {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.vorbis);
        buf.p1(self.loops);
        buf.p2(self.delay);
        buf.p1(self.volume);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.vorbis)
            + size_of_val(&self.loops)
            + size_of_val(&self.delay)
            + size_of_val(&self.volume)
    }
}
