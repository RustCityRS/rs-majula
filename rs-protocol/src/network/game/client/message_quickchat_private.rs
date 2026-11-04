//! `MessageQuickchatPrivate` — rev-647 MESSAGE_QUICKCHAT_PRIVATE (51, var_byte).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(VarByte { min: 0 }, UserEvent)]
pub struct MessageQuickchatPrivate {
    pub target: String,
    pub payload: Vec<u8>,
}

impl ClientProtMessage for MessageQuickchatPrivate {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let target = buf.gjstr(0);
        let mut payload = Vec::with_capacity(buf.remaining() as usize);
        while buf.remaining() > 0 {
            payload.push(buf.g1());
        }
        MessageQuickchatPrivate { target, payload }
    }
}
