use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(Fixed(10), UserEvent)]
pub struct SendSnapshot {
    pub offender: i64,
    pub reason: u8,
    pub mute: bool,
}

#[cfg(before_647)]
impl ClientProtMessage for SendSnapshot {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        SendSnapshot {
            offender: buf.g8s(),
            reason: buf.g1(),
            mute: buf.g1() != 0,
        }
    }
}

/// Rev-647 SEND_SNAPSHOT, opcode 3: an abuse report, by name. CS2 command
/// 5002 (`Class47`): `pjstr(offender) p1(rule - 1) p1(mute) pjstr(text)`,
/// `text` a second string off the script stack, cut to 80 characters.
#[cfg(since_647)]
#[client_prot(VarByte { min: 0 }, UserEvent)]
pub struct SendSnapshot {
    pub offender: String,
    pub reason: u8,
    pub mute: bool,
    pub text: String,
}

#[cfg(since_647)]
impl ClientProtMessage for SendSnapshot {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        SendSnapshot {
            offender: buf.gjstr(0),
            reason: buf.g1(),
            mute: buf.g1() != 0,
            text: buf.gjstr(0),
        }
    }
}
