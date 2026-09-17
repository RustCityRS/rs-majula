use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(8), UserEvent)]
pub struct OpObjT {
    pub x: u16,
    pub z: u16,
    pub obj: u16,
    pub com: u16,
}

impl ClientProtMessage for OpObjT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpObjT {
            x: buf.g2(),
            z: buf.g2(),
            obj: buf.g2(),
            com: buf.g2(),
        }
    }
}
