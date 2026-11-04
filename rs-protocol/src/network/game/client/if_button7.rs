use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(8), UserEvent)]
pub struct IfButton7 {
    pub com: u32,
    pub slot: i16,
    pub obj: i16,
}

impl ClientProtMessage for IfButton7 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4s() as u32;
        let obj = buf.ig2_alt1() as i16;
        let slot = buf.ig2() as i16;
        IfButton7 { com, slot, obj }
    }
}
