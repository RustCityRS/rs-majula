use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(2), UserEvent)]
pub struct OpNpc5 {
    pub nid: u16,
}

impl ClientProtMessage for OpNpc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpNpc5 { nid: buf.g2() }
    }
}
