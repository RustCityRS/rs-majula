use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(12), UserEvent)]
pub struct OpLocU {
    pub x: u16,
    pub z: u16,
    pub loc: u16,
    pub obj: u16,
    pub slot: u16,
    pub com: u16,
}

impl ClientProtMessage for OpLocU {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpLocU {
            x: buf.g2(),
            z: buf.g2(),
            loc: buf.g2(),
            obj: buf.g2(),
            slot: buf.g2(),
            com: buf.g2(),
        }
    }
}
