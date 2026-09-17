use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(10), UserEvent)]
pub struct SendSnapshot {
    pub offender: i64,
    pub reason: u8,
    pub mute: bool,
}

impl ClientProtMessage for SendSnapshot {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        SendSnapshot {
            offender: buf.g8s(),
            reason: buf.g1(),
            mute: buf.g1() != 0,
        }
    }
}
