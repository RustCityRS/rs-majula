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

// `SynthSound` — play a synthesized sound effect.
// Rev-647 opcode 50 (SYNTH_SOUND), fixed 8.
//
// The rev-647 client reads five fields in order (see MapAreaConfig.java):
// `g2 synth_id; g1 loops; g2 delay; g1 volume; g2 speed`. Field meanings
// confirmed against void-server's rev-634 `SoundEncoder.kt` (identical
// layout: sound/repeat/delay/volume/speed, defaults volume=255 speed=255
// repeat=1). The client DISCARDS the sound when `loops == 0`, and plays
// at `volume` (0 = mute). 65535 in `synth_id` is the client's -1 sentinel.

#[cfg(since_647)]
#[server_prot(SynthSound, Buffered, Fixed)]
pub struct SynthSound {
    pub synth_id: u16,
    pub loops: u8,
    pub delay: u16,
    pub volume: u8,
    pub speed: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for SynthSound {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.synth_id);
        buf.p1(self.loops);
        buf.p2(self.delay);
        buf.p1(self.volume);
        buf.p2(self.speed);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.synth_id)
            + size_of_val(&self.loops)
            + size_of_val(&self.delay)
            + size_of_val(&self.volume)
            + size_of_val(&self.speed)
    }
}
