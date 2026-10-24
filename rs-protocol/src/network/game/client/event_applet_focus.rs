#[cfg(since_254)]
use crate::network::game::client::ClientProtMessage;
#[cfg(since_254)]
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
#[cfg(since_254)]
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_254)]
use rs_io::Packet;
#[cfg(since_254)]
use rs_protocol_macros::client_prot;

/// The client's window gained or lost focus.
#[cfg(since_254)]
#[client_prot(Fixed(1), ClientEvent)]
pub struct EventAppletFocus {
    pub focused: bool,
}

#[cfg(since_254)]
impl ClientProtMessage for EventAppletFocus {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        EventAppletFocus {
            focused: buf.g1() == 1,
        }
    }
}
