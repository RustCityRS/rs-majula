use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(4), ClientEvent)]
pub struct SoundSongPreloaded {
    pub id: i32,
}

impl ClientProtMessage for SoundSongPreloaded {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        SoundSongPreloaded { id: buf.g4s() }
    }
}
