use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(4), ClientEvent)]
pub struct WorldlistFetch {
    pub checksum: i32,
}

impl ClientProtMessage for WorldlistFetch {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        WorldlistFetch {
            checksum: buf.g4s(),
        }
    }
}
