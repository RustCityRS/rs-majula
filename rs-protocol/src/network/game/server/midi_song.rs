use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(rev = "225")]
use crate::network::game::server_prot_message::jstr_len;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

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
        buf.pjstr(self.name, 10);
        buf.p4(self.crc);
        buf.p4(self.len);
    }

    fn sizeof(&self) -> usize {
        jstr_len(self.name) + size_of_val(&self.crc) + size_of_val(&self.len)
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

// `MidiSong` — play a MIDI song (the map-loop music).
// Rev-647 opcode 15 (MIDI_SONG), fixed 4.
// Wire layout: `[p1_alt2 volume][ip2_alt1 song_id][p1_alt3 delay]`.

#[cfg(since_647)]
#[server_prot(MidiSong, Buffered, Fixed)]
pub struct MidiSong {
    pub song_id: u16,
    pub volume: u8,
    pub delay: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for MidiSong {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.volume);
        buf.ip2_alt1(self.song_id);
        buf.p1_alt3(self.delay);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.song_id) + size_of_val(&self.volume) + size_of_val(&self.delay)
    }
}
