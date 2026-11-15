use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(all(since_530, before_647))]
#[client_prot(Fixed(4), ClientEvent)]
pub struct ClientDetailoptionsStatus {
    pub options: i32,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for ClientDetailoptionsStatus {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ClientDetailoptionsStatus { options: buf.g4s() }
    }
}

#[cfg(since_647)]
#[client_prot(VarByte { min: 1 }, ClientEvent)]
pub struct ClientDetailoptionsStatus {
    pub version: u8,
    pub bytes: Vec<u8>,
}

#[cfg(since_647)]
impl ClientProtMessage for ClientDetailoptionsStatus {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let version = buf.g1();
        let bytes = buf.data[buf.pos..len].to_vec();
        buf.pos = len;
        ClientDetailoptionsStatus { version, bytes }
    }
}
