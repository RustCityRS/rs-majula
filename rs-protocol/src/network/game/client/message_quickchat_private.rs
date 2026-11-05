use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::{ClientProtMessageInfo, string_end};
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::STRING_TERMINATOR;

#[client_prot(VarByte { min: 3 }, UserEvent)]
pub struct MessageQuickchatPrivate {
    pub name: String,
    pub id: u16,
    pub bytes: Vec<u8>,
}

impl ClientProtMessage for MessageQuickchatPrivate {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0).is_some_and(|end| end + 2 <= data.len())
    }

    fn decode(buf: &mut Packet, len: usize) -> Self {
        let name = buf.gjstr(STRING_TERMINATOR);
        let id = buf.g2();
        let bytes = buf.data[buf.pos..len].to_vec();
        buf.pos = len;
        MessageQuickchatPrivate { name, id, bytes }
    }
}
