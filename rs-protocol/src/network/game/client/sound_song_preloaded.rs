//! `SoundSongPreloaded` — rev-647 SOUND_SONGPRELOADED (opcode 59, fixed 4).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(4), ClientEvent)]
pub struct SoundSongPreloaded {
    pub song_id: i32,
}

impl ClientProtMessage for SoundSongPreloaded {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        SoundSongPreloaded { song_id: buf.g4s() }
    }
}
