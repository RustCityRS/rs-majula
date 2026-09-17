use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(6), UserEvent)]
pub struct OpLoc5 {
    pub x: u16,
    pub z: u16,
    pub loc: u16,
}

impl ClientProtMessage for OpLoc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpLoc5 {
            x: buf.g2(),
            z: buf.g2(),
            loc: buf.g2(),
        }
    }
}
