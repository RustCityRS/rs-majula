//! `MessageQuickchatPublic` — rev-647 MESSAGE_QUICKCHAT_PUBLIC (54, var_byte).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(VarByte { min: 0 }, UserEvent)]
pub struct MessageQuickchatPublic {
    pub payload: Vec<u8>,
}

impl ClientProtMessage for MessageQuickchatPublic {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let mut payload = Vec::with_capacity(buf.remaining() as usize);
        while buf.remaining() > 0 {
            payload.push(buf.g1());
        }
        MessageQuickchatPublic { payload }
    }
}
