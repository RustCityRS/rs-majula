//! `ChatSetMode` — rev-647 CHAT_SETMODE (opcode 61, fixed 1).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(1), UserEvent)]
pub struct ChatSetMode {
    pub mode: u8,
}

impl ClientProtMessage for ChatSetMode {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ChatSetMode { mode: buf.g1() }
    }
}
