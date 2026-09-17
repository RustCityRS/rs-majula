use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(13), UserEvent)]
pub struct IdkSaveDesign {
    pub gender: u8,
    pub idkit: Vec<u8>,
    pub colour: Vec<u8>,
}

impl ClientProtMessage for IdkSaveDesign {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        IdkSaveDesign {
            gender: buf.g1(),
            idkit: (0..7).map(|_| buf.g1()).collect(),
            colour: (0..5).map(|_| buf.g1()).collect(),
        }
    }
}
