use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_647)]
use crate::network::game::client_prot_message::string_end;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
#[cfg(since_647)]
use rs_ver::STRING_TERMINATOR;

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
#[cfg(since_647)]
#[client_prot(VarByte { min: 4 }, UserEvent)]
pub struct SendSnapshot {
    pub offender: String,
    pub reason: u8,
    pub mute: bool,
    pub text: String,
}

#[cfg(since_647)]
impl ClientProtMessage for SendSnapshot {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0)
            .and_then(|end| string_end(data, end + 2))
            .is_some_and(|end| end == data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        SendSnapshot {
            offender: buf.gjstr(STRING_TERMINATOR),
            reason: buf.g1(),
            mute: buf.g1() != 0,
            text: buf.gjstr(STRING_TERMINATOR),
        }
    }
}
