use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(6), UserEvent)]
pub struct OpHeld3 {
    pub obj: u16,
    pub slot: u16,
    pub com: u16,
}

impl ClientProtMessage for OpHeld3 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpHeld3 {
            obj: buf.g2(),
            slot: buf.g2(),
            com: buf.g2(),
        }
    }
}
