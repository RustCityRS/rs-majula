use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(16), UserEvent)]
pub struct IfButtonT {
    pub com: u32,
    pub slot: u16,
    pub obj: u16,
    pub com2: u32,
    pub slot2: u16,
    pub obj2: u16,
}

impl ClientProtMessage for IfButtonT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com2 = buf.g4s() as u32;
        let slot2 = buf.ig2();
        let slot = buf.ig2();
        let obj = buf.g2_alt1();
        let com = buf.ig4s() as u32;
        let obj2 = buf.g2();
        IfButtonT {
            com,
            slot,
            obj,
            com2,
            slot2,
            obj2,
        }
    }
}
