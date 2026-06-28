#[cfg(since_254)]
use crate::network::game::client::ClientProtMessage;
#[cfg(since_254)]
use crate::network::game::client_prot_category::ClientProtCategory;
#[cfg(since_254)]
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_254)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_254)]
use rs_protocol_macros::client_prot;

#[cfg(since_254)]
#[client_prot(Fixed(2), UserEvent)]
pub struct OpPlayer5 {
    pub pid: u16,
}

#[cfg(since_254)]
impl ClientProtMessage for OpPlayer5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayer5 { pid: buf.g2() }
    }
}
