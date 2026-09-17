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
#[client_prot(Fixed(0), ClientEvent)]
pub struct AnticheatCycleLogic7;

#[cfg(since_254)]
impl ClientProtMessage for AnticheatCycleLogic7 {
    fn decode(_: &mut Packet, _: usize) -> Self {
        AnticheatCycleLogic7
    }
}
