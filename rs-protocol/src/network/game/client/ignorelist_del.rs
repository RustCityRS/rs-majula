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
#[client_prot(Fixed(8), UserEvent)]
pub struct IgnoreListDel {
    pub user37: i64,
}

#[cfg(before_647)]
impl ClientProtMessage for IgnoreListDel {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        IgnoreListDel { user37: buf.g8s() }
    }
}

#[cfg(since_647)]
#[client_prot(VarByte { min: 1 }, UserEvent)]
pub struct IgnoreListDel {
    pub name: String,
}

#[cfg(since_647)]
impl ClientProtMessage for IgnoreListDel {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0) == Some(data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        IgnoreListDel {
            name: buf.gjstr(STRING_TERMINATOR),
        }
    }
}
