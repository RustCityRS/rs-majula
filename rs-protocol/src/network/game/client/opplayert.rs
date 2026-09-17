use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(4), UserEvent)]
pub struct OpPlayerT {
    pub pid: u16,
    pub com: u16,
}

impl ClientProtMessage for OpPlayerT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayerT {
            pid: buf.g2(),
            com: buf.g2(),
        }
    }
}
