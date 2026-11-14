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

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for FaceSquare {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let z = buf.g2();
        let x = buf.g2();
        FaceSquare { x, z }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for FaceSquare {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let (x, z) = (buf.g2_alt2(), buf.g2_alt1());
        #[cfg(since_742)]
        let (z, x) = (buf.g2_alt3(), buf.g2());
        FaceSquare { x, z }
    }
}
