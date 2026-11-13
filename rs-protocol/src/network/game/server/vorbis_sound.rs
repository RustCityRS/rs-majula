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

#[server_prot(VorbisSound, Buffered, Fixed)]
pub struct VorbisSound {
    pub sample_id: u16,
    pub loops: u8,
    pub delay: u16,
    pub unk1: u8,
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
        size_of_val(&self.sample_id)
            + size_of_val(&self.loops)
            + size_of_val(&self.delay)
            + size_of_val(&self.unk1)
            + size_of_val(&self.unk2)
    }
}
