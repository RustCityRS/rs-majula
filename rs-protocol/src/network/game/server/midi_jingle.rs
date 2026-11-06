use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(rev = "225")]
#[server_prot(MidiJingle, Buffered, VarShort)]
pub struct MidiJingle<'a> {
    pub delay: u16,
    pub bytes: &'a [u8],
}

#[cfg(rev = "225")]
impl ServerProtMessage for MidiJingle<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.delay);
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.delay) + self.bytes.len()
    }
}

#[cfg(all(since_244, before_647))]
#[server_prot(MidiJingle, Buffered, Fixed)]
pub struct MidiJingle {
    pub id: u16,
    pub delay: u16,
}

#[cfg(all(since_244, before_647))]
impl ServerProtMessage for MidiJingle {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.id);
        buf.p2(self.delay);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.delay)
    }
}

#[cfg(since_647)]
#[server_prot(MidiJingle, Buffered, Fixed)]
pub struct MidiJingle {
    pub song_id: u16,
    pub duration: u32,
    pub volume: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for MidiJingle {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt3(self.song_id);
        buf.p3_alt2(self.duration);
        buf.p1(self.volume);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.song_id) + 3 + size_of_val(&self.volume)
    }
}
