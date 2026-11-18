use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(4), ClientEvent)]
pub struct DetectModifiedClient {
    pub id: i32,
}

impl ClientProtMessage for DetectModifiedClient {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        DetectModifiedClient { id: buf.g4s() }
    }
}
