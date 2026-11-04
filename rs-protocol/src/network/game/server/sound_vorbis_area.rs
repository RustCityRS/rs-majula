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
#[server_prot(SoundVorbisArea, Buffered, Fixed)]
pub struct SoundVorbisArea {
    pub coord: u8,
    pub sample_id: u16,
    pub radius_rate: u8,
    pub unk_c: u8,
    pub unk_d: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for SoundVorbisArea {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.coord);
        buf.p2(self.sample_id);
        buf.p1(self.radius_rate);
        buf.p1(self.unk_c);
        buf.p1(self.unk_d);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.coord)
            + size_of_val(&self.sample_id)
            + size_of_val(&self.radius_rate)
            + size_of_val(&self.unk_c)
            + size_of_val(&self.unk_d)
    }
}
