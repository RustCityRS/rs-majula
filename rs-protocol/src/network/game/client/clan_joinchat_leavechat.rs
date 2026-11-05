use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::{ClientProtMessageInfo, string_end};
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::STRING_TERMINATOR;

#[client_prot(VarByte { min: 0 }, UserEvent)]
pub struct ClanJoinchatLeavechat {
    pub name: String,
}

impl ClientProtMessage for ClanJoinchatLeavechat {
    fn validate(data: &[u8]) -> bool {
        data.is_empty() || string_end(data, 0) == Some(data.len())
    }

    fn decode(buf: &mut Packet, len: usize) -> Self {
        ClanJoinchatLeavechat {
            name: if len == 0 {
                String::new()
            } else {
                buf.gjstr(STRING_TERMINATOR)
            },
        }
    }
}
