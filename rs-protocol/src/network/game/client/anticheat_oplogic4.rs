use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::client_prot;

#[cfg_attr(before_254, client_prot(Fixed(2), ClientEvent))]
#[cfg_attr(since_254, client_prot(Fixed(1), ClientEvent))]
pub struct AnticheatOpLogic4;

impl ClientProtMessage for AnticheatOpLogic4 {
    fn decode(_: &mut Packet, _: usize) -> Self {
        AnticheatOpLogic4
    }
}
