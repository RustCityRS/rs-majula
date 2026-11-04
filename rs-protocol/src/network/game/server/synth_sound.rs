use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(SynthSound, Buffered, Fixed)]
pub struct SynthSound {
    pub synth: u16,
    pub loops: u8,
    pub delay: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for SynthSound {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.synth);
        buf.p1(self.loops);
        buf.p2(self.delay);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.synth) + size_of_val(&self.loops) + size_of_val(&self.delay)
    }
}

#[cfg(since_647)]
#[server_prot(SynthSound, Buffered, Fixed)]
pub struct SynthSound {
    pub synth: u16,
    pub loops: u8,
    pub delay: u16,
    pub volume: u8,
    pub speed: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for SynthSound {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.synth);
        buf.p1(self.loops);
        buf.p2(self.delay);
        buf.p1(self.volume);
        buf.p2(self.speed);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.synth)
            + size_of_val(&self.loops)
            + size_of_val(&self.delay)
            + size_of_val(&self.volume)
            + size_of_val(&self.speed)
    }
}
