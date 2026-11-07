use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(VarByte { min: 8 }, ClientEvent)]
pub struct ReflectionCheckReply {
    pub id: i32,
    pub bytes: Vec<u8>,
    pub crc: i32,
}

impl ClientProtMessage for ReflectionCheckReply {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let id = buf.g4s();
        let bytes = buf.data[buf.pos..len - 4].to_vec();
        buf.pos = len - 4;
        let crc = buf.g4s();
        ReflectionCheckReply { id, bytes, crc }
    }
}
