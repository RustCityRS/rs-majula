use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(4), UserEvent)]
pub struct FaceSquare {
    pub x: u16,
    pub z: u16,
}

impl ClientProtMessage for FaceSquare {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2_alt1();
        let z = buf.ig2();
        FaceSquare { x, z }
    }
}
