use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_647)]
use crate::network::game::client_prot_message::string_end;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(Fixed(8), UserEvent)]
pub struct FriendListDel {
    pub user37: i64,
}

#[cfg(before_647)]
impl ClientProtMessage for FriendListDel {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        FriendListDel { user37: buf.g8s() }
    }
}

#[cfg(since_647)]
#[client_prot(VarByte { min: 1 }, UserEvent)]
pub struct FriendListDel {
    pub name: String,
}

#[cfg(since_647)]
impl ClientProtMessage for FriendListDel {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0) == Some(data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        FriendListDel { name: buf.gjstr(0) }
    }
}
