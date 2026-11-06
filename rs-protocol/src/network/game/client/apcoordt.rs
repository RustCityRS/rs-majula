use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(12), UserEvent)]
pub struct ApCoordT {
    pub x: u16,
    pub z: u16,
    pub obj: u16,
    pub slot: u16,
    pub com: u32,
}

impl ClientProtMessage for ApCoordT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2_alt1();
        let slot = buf.g2();
        let obj = buf.g2_alt2();
        let z = buf.g2_alt2();
        let com = buf.g4s() as u32;
        ApCoordT {
            x,
            z,
            obj,
            slot,
            com,
        }
    }
}
