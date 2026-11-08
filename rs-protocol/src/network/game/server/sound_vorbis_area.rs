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
#[server_prot(SoundVorbisArea, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct SoundVorbisArea {
    pub coord: u8,
    pub sound: u16,
    pub info: u8,
    pub delay: u8,
    pub volume: u8,
    pub speed: u16,
}

#[cfg(since_647)]
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
