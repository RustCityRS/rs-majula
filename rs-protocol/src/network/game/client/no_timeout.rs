use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(Fixed(0), ClientEvent)]
pub struct NoTimeout;

#[cfg(before_647)]
impl ClientProtMessage for NoTimeout {
    fn decode(_: &mut Packet, _: usize) -> Self {
        NoTimeout
    }
}

// `NoTimeout` — rev-647 NO_TIMEOUT (opcode 0, fixed 0).

#[cfg(since_647)]
#[client_prot(Fixed(0), ClientEvent)]
pub struct NoTimeout;

#[cfg(since_647)]
impl ClientProtMessage for NoTimeout {
    fn decode(_buf: &mut Packet, _: usize) -> Self {
        NoTimeout
    }
}
