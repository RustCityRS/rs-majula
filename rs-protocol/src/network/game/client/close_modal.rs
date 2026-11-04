use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(Fixed(0), UserEvent)]
pub struct CloseModal;

#[cfg(before_647)]
impl ClientProtMessage for CloseModal {
    fn decode(_: &mut Packet, _: usize) -> Self {
        CloseModal
    }
}

// `CloseModal` — rev-647 CLOSE_MODAL (opcode 42, fixed 0).

#[cfg(since_647)]
#[client_prot(Fixed(0), UserEvent)]
pub struct CloseModal;

#[cfg(since_647)]
impl ClientProtMessage for CloseModal {
    fn decode(_buf: &mut Packet, _: usize) -> Self {
        CloseModal
    }
}
