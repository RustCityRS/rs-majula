use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::{ClientProtMessageInfo, string_end};
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::STRING_TERMINATOR;

#[client_prot(VarByte { min: 4 }, UserEvent)]
pub struct ClanchannelKickUser {
    pub affined: bool,
    pub index: u16,
    pub name: String,
}

impl ClientProtMessage for ClanchannelKickUser {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 3) == Some(data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        ClanchannelKickUser {
            affined: buf.g1() != 0,
            index: buf.g2(),
            name: buf.gjstr(STRING_TERMINATOR),
        }
    }
}
