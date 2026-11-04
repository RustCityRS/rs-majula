//! `SoundVorbisArea` — standalone area vorbis sound.
//! Rev-647 opcode 72 (SOUND_VORBIS_AREA), fixed 6.
//!
//! NOTE: ambiguous layout. The rev-647 top-level dispatcher hands off to
//! the zone-sub-packet handler (`AClass17.method825` / `ZoneProt.SOUND_VORBIS_AREA`),
//! which reads 8 bytes (`g1 coord; g2 sample_id; g1 radius_rate; g1 c; g1 d; g2 delay`).
//! The standalone opcode's declared size is 6, so we emit a 6-byte prefix
//! matching the first six client reads: `g1 coord; g2 sample_id; g1 radius_rate; g1 c; g1 d`.

use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(SoundVorbisArea, Buffered, Fixed)]
pub struct SoundVorbisArea {
    /// Packed `(local_x << 4) | local_z`.
    pub coord: u8,
    /// Vorbis sample id (65535 → -1).
    pub sample_id: u16,
    /// Packed radius (hi nibble) and rate (lo nibble).
    pub radius_rate: u8,
    /// Opaque (client `var151`).
    pub unk_c: u8,
    /// Opaque (client `var139`).
    pub unk_d: u8,
}

impl ServerProtMessage for SoundVorbisArea {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.sample_id);
        buf.p1(self.radius_rate);
        buf.p1(self.unk_c);
        buf.p1(self.unk_d);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.coord)
            + size_of_val(&self.sample_id)
            + size_of_val(&self.radius_rate)
            + size_of_val(&self.unk_c)
            + size_of_val(&self.unk_d)
    }
}
