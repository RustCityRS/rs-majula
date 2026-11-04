//! `UrlRequest` — rev-647 URL_REQUEST (opcode 27, var_byte).
//!
//! CS2 command 5400: `pjstr pjstr p1`, two strings and an int off the script
//! stacks (`Class47`). Kept as its frame.
use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct UrlRequest {
    pub payload: Vec<u8>,
}

impl ClientProtMessage for UrlRequest {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let payload = (0..len).map(|_| buf.g1()).collect();
        UrlRequest { payload }
    }
}
