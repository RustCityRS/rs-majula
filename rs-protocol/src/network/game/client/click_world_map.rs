use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(4), UserEvent)]
pub struct ClickWorldMap {
    pub coord: u32,
}

impl ClientProtMessage for ClickWorldMap {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ClickWorldMap {
            coord: buf.g4_alt2() as u32,
        }
    }
}
