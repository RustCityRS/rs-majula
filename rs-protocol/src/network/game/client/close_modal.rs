use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(0), UserEvent)]
pub struct CloseModal;

impl ClientProtMessage for CloseModal {
    fn decode(_: &mut Packet, _: usize) -> Self {
        CloseModal
    }
}
