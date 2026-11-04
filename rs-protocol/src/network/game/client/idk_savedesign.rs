use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use std::array::from_fn;

#[client_prot(Fixed(13), UserEvent)]
pub struct IdkSaveDesign {
    pub gender: u8,
    pub idkit: [u8; 7],
    pub colour: [u8; 5],
}

impl ClientProtMessage for IdkSaveDesign {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        IdkSaveDesign {
            gender: buf.g1(),
            idkit: from_fn(|_| buf.g1()),
            colour: from_fn(|_| buf.g1()),
        }
    }
}
