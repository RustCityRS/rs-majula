use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[client_prot(Fixed(8), UserEvent)]
pub struct OpHeldT {
    pub obj: TypeId,
    pub slot: u16,
    pub com: u16,
    pub com2: u16,
}

impl ClientProtMessage for OpHeldT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpHeldT {
            obj: TypeId::from_g2(buf.g2()),
            slot: buf.g2(),
            com: buf.g2(),
            com2: buf.g2(),
        }
    }
}
