//! `VorbisSound` — play a vorbis sample (one-shot).
//! Rev-647 opcode 51 (VORBIS_SOUND), fixed 8.
//!
//! Client read order (`MapAreaConfig.java`):
//! `g2 sample_id; g1 loops; g2 delay; g1 unk1; g2 unk2`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(VorbisSound, Buffered, Fixed)]
pub struct VorbisSound {
    /// Vorbis sample id (65535 → -1 sentinel).
    pub sample_id: u16,
    /// Loop count byte.
    pub loops: u8,
    /// Delay before playback (client ticks).
    pub delay: u16,
    /// Opaque tag byte (client `var202`).
    pub unk1: u8,
    /// Opaque word (client `var158`).
    pub unk2: u16,
}

impl ServerProtMessage for VorbisSound {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.sample_id);
        buf.p1(self.loops);
        buf.p2(self.delay);
        buf.p1(self.unk1);
        buf.p2(self.unk2);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.sample_id)
            + size_of_val(&self.loops)
            + size_of_val(&self.delay)
            + size_of_val(&self.unk1)
            + size_of_val(&self.unk2)
    }
}
