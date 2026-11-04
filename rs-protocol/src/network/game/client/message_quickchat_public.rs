use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(VarByte { min: 3 }, UserEvent)]
pub struct MessageQuickchatPublic {
    pub chat: u8,
    pub id: u16,
    pub bytes: Vec<u8>,
}

impl ClientProtMessage for MessageQuickchatPublic {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let chat = buf.g1();
        let id = buf.g2();
        let bytes = buf.data[buf.pos..len].to_vec();
        buf.pos = len;
        MessageQuickchatPublic { chat, id, bytes }
    }
}
