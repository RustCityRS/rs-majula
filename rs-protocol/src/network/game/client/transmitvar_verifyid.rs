//! `TransmitvarVerifyId` — rev-647 TRANSMITVAR_VERIFYID (opcode 89, fixed 2).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(Fixed(2), ClientEvent)]
pub struct TransmitvarVerifyId {
    pub id: u16,
}

impl ClientProtMessage for TransmitvarVerifyId {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        TransmitvarVerifyId { id: buf.g2() }
    }
}
