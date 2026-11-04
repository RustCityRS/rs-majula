use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(2), ClientEvent)]
pub struct TransmitvarVerifyId {
    pub id: u16,
}

impl ClientProtMessage for TransmitvarVerifyId {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        TransmitvarVerifyId { id: buf.g2() }
    }
}
