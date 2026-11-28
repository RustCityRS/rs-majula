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

#[client_prot(Fixed(0), UserEvent)]
#[derive(Debug)]
pub struct CloseModal;

impl ClientProtMessage for CloseModal {
    fn decode(_: &mut Packet, _: usize) -> Self {
        CloseModal
    }
}

#[cfg(since_530)]
impl ClientProtEncode for CloseModal {
    const PROT: ClientProt = ClientProt::CloseModal;

    fn encode(&self, _: &mut Packet) {}
}
