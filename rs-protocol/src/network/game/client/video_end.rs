//! `VideoEnd` — rev-647 opcode 23 (fixed 2).
//!
//! A video the client was playing has ended (`IntegerNode.method3202`):
//! `p2(id)` when the server is to be told, otherwise the client runs its own
//! VIDEO_END trigger instead.
use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(2), ClientEvent)]
pub struct VideoEnd {
    pub id: u16,
}

impl ClientProtMessage for VideoEnd {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        VideoEnd { id: buf.g2() }
    }
}
