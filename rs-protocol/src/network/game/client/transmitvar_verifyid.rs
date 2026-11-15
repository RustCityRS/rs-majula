use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg_attr(before_742, client_prot(Fixed(2), ClientEvent))]
#[cfg_attr(since_742, client_prot(Fixed(4), ClientEvent))]
pub struct TransmitvarVerifyId {
    pub id: i32,
}

impl ClientProtMessage for TransmitvarVerifyId {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let id = buf.g2() as i32;
        #[cfg(since_742)]
        let id = buf.g4s();
        TransmitvarVerifyId { id }
    }
}
