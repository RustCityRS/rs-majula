use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[client_prot(Fixed(12), UserEvent)]
pub struct OpHeldU {
    pub obj: TypeId,
    pub slot: u16,
    pub com: u16,
    pub obj2: TypeId,
    pub slot2: u16,
    pub com2: u16,
}

impl ClientProtMessage for OpHeldU {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpHeldU {
            obj: TypeId::from_g2(buf.g2()),
            slot: buf.g2(),
            com: buf.g2(),
            obj2: TypeId::from_g2(buf.g2()),
            slot2: buf.g2(),
            com2: buf.g2(),
        }
    }
}
