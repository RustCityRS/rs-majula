use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::client_prot;

#[cfg_attr(rev = "225", client_prot(Fixed(6), ClientEvent))]
#[cfg_attr(since_254, client_prot(Fixed(4), ClientEvent))]
pub struct EventCameraPosition;

impl ClientProtMessage for EventCameraPosition {
    fn decode(_: &mut Packet, _: usize) -> Self {
        EventCameraPosition
    }
}
