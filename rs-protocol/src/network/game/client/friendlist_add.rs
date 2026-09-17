use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(8), UserEvent)]
pub struct FriendListAdd {
    pub user37: i64,
}

impl ClientProtMessage for FriendListAdd {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        FriendListAdd { user37: buf.g8s() }
    }
}
