#[cfg(since_530)]
use crate::network::game::client::ClientProtEncode;
use crate::network::game::client::ClientProtMessage;
#[cfg(since_530)]
use crate::network::game::client_prot::ClientProt;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(0), ClientEvent)]
#[derive(Debug)]
pub struct NoTimeout;

impl ClientProtMessage for NoTimeout {
    fn decode(_: &mut Packet, _: usize) -> Self {
        NoTimeout
    }
}

#[cfg(since_530)]
impl ClientProtEncode for NoTimeout {
    const PROT: ClientProt = ClientProt::NoTimeout;

    fn encode(&self, _: &mut Packet) {}
}
