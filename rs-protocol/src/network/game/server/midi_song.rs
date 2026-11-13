use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
#[cfg(rev = "225")]
use rs_ver::STRING_TERMINATOR;

#[cfg(rev = "225")]
#[server_prot(MidiSong, Buffered, VarByte)]
pub struct MidiSong<'a> {
    pub name: &'a str,
    pub crc: i32,
    pub len: i32,
}

#[cfg(rev = "225")]
impl ServerProtMessage for MidiSong<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(self.name, STRING_TERMINATOR);
        buf.p4(self.crc);
        buf.p4(self.len);
    }

    fn sizeof(&self) -> usize {
        Packet::sizejstr(self.name) + size_of_val(&self.crc) + size_of_val(&self.len)
    }
}

#[cfg(all(since_244, before_647))]
#[server_prot(MidiSong, Buffered, Fixed)]
pub struct MidiSong {
    pub id: u16,
}

#[cfg(all(since_244, before_647))]
impl ServerProtMessage for MidiSong {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id)
    }
}

#[cfg(since_647)]
#[server_prot(MidiSong, Buffered, Fixed)]
pub struct MidiSong {
    pub id: u16,
    pub volume: u8,
    pub delay: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for MidiSong {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt3(self.volume);
        buf.p2_alt3(self.id);
        buf.p1_alt1(self.delay);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.volume) + size_of_val(&self.delay)
    }
}
