use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(8), UserEvent)]
pub struct IfButton10 {
    pub com: i32,
    pub slot: u16,
    pub obj: u16,
}

impl ClientProtMessage for IfButton10 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4s();
        let obj = buf.g2_alt3();
        let slot = buf.g2_alt1();
        IfButton10 { com, slot, obj }
    }
}
