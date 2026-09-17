use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(1), UserEvent)]
pub struct TutClickSide {
    pub tab: u8,
}

impl ClientProtMessage for TutClickSide {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        TutClickSide { tab: buf.g1() }
    }
}
