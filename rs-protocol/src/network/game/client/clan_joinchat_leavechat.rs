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

#[cfg(all(since_530, before_647))]
#[client_prot(Fixed(8), UserEvent)]
pub struct ClanJoinchatLeavechat {
    pub user37: i64,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for ClanJoinchatLeavechat {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ClanJoinchatLeavechat { user37: buf.g8s() }
    }
}

#[cfg(since_647)]
#[client_prot(VarByte { min: 0 }, UserEvent)]
pub struct ClanJoinchatLeavechat {
    pub name: String,
}

#[cfg(since_647)]
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
