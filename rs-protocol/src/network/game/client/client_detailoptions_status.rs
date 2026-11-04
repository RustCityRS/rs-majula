//! `ClientDetailoptionsStatus` — rev-647 CLIENT_DETAILOPTIONS_STATUS
//! (opcode 57, var_byte).
//!
//! The client's graphics and sound options, `ClientOptions.encode`, sent once
//! per session (`VarbitType`). Kept as its frame.
use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(VarByte { min: 0 }, ClientEvent)]
pub struct ClientDetailoptionsStatus {
    pub payload: Vec<u8>,
}

impl ClientProtMessage for ClientDetailoptionsStatus {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let payload = (0..len).map(|_| buf.g1()).collect();
        ClientDetailoptionsStatus { payload }
    }
}
