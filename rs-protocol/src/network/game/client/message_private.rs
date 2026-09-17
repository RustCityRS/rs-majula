use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

// Minimum covers the base37 username.
#[client_prot(VarByte { min: 8 }, UserEvent)]
pub struct MessagePrivate {
    pub user37: i64,
    pub bytes: Vec<u8>,
}

impl ClientProtMessage for MessagePrivate {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let user37 = buf.g8s();
        let mut bytes = vec![0; len - buf.pos];
        buf.gdata(&mut bytes, 0, len - buf.pos);
        MessagePrivate { user37, bytes }
    }
}
