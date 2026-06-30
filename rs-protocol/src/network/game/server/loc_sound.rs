#[cfg(since_289)]
use crate::network::game::server::ServerProtMessage;
#[cfg(since_289)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_289)]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(since_289)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_289)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_289)]
use rs_protocol_macros::server_prot;

/// Plays a sound effect anchored to a tile within a zone.
///
/// `coord` is the packed zone-local tile offset (`x << 4 | z`). The wire byte
/// after the sound id packs `range` into bits 4-7 and `loops` into bits 0-2.
#[cfg(since_289)]
#[server_prot(SoundArea, Immediate, Fixed)]
pub struct SoundArea {
    pub coord: u8,
    pub sound: u16,
    pub info: u8,
}

#[cfg(since_289)]
impl ServerProtMessage for SoundArea {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.sound);
        buf.p1(self.info);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord) + size_of_val(&self.sound) + size_of_val(&self.info)
    }
}
