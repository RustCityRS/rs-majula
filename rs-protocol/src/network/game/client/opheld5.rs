use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[client_prot(Fixed(6), UserEvent)]
pub struct OpHeld5 {
    pub obj: TypeId,
    pub slot: u16,
    pub com: u16,
}

impl ClientProtMessage for OpHeld5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpHeld5 {
            obj: TypeId::from_g2(buf.g2()),
            slot: buf.g2(),
            com: buf.g2(),
        }
    }
}
