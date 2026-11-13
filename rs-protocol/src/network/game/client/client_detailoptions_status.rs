use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(VarByte { min: 1 }, ClientEvent)]
pub struct ClientDetailoptionsStatus {
    pub version: u8,
    pub bytes: Vec<u8>,
}

impl ClientProtMessage for ClientDetailoptionsStatus {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let version = buf.g1();
        let bytes = buf.data[buf.pos..len].to_vec();
        buf.pos = len;
        ClientDetailoptionsStatus { version, bytes }
    }
}
