#[cfg(all(since_647, before_742))]
use crate::network::game::server_prot::ServerProt;
#[cfg(all(since_647, before_742))]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(all(since_647, before_742))]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::Packet;
#[cfg(all(since_647, before_742))]
use rs_io::PacketFrame;
#[cfg(all(since_647, before_742))]
use rs_protocol_macros::server_prot;

#[cfg_attr(
    all(since_647, before_742),
    server_prot(SoundVorbisArea, Immediate, Fixed)
)]
#[cfg(since_647)]
#[derive(Debug, Clone)]
pub struct SoundVorbisArea {
    pub coord: u8,
    pub sound: u16,
    pub info: u8,
    pub delay: u8,
    pub volume: u8,
    pub speed: u16,
    #[cfg(since_742)]
    pub unknown: u8,
}

#[cfg(all(since_647, before_742))]
impl ServerProtMessage for SoundVorbisArea {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.sound);
        buf.p1(self.info);
        buf.p1(self.delay);
        buf.p1(self.volume);
        buf.p2(self.speed);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.sound)
            + size_of_val(&self.info)
            + size_of_val(&self.delay)
            + size_of_val(&self.volume)
            + size_of_val(&self.speed)
    }
}

// 742 sends it only inside UPDATE_ZONE_PARTIAL_ENCLOSED, so it has no prot of its own.
#[cfg(since_742)]
impl SoundVorbisArea {
    pub fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.sound);
        buf.p1(self.info);
        buf.p1(self.delay);
        buf.p1(self.volume);
        buf.p2(self.speed);
        buf.p1(self.unknown);
    }

    pub fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.sound)
            + size_of_val(&self.info)
            + size_of_val(&self.delay)
            + size_of_val(&self.volume)
            + size_of_val(&self.speed)
            + size_of_val(&self.unknown)
    }
}
