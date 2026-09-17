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

#[cfg(since_254)]
#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct EventMouseMove;

#[cfg(since_254)]
impl ClientProtMessage for EventMouseMove {
    fn decode(_: &mut Packet, _: usize) -> Self {
        EventMouseMove
    }
}
