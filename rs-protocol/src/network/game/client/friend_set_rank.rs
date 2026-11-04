//! `FriendSetRank` — rev-647 FRIEND_SETRANK (opcode 77, var_byte).

use crate::network::game::client_prot_frame::ClientProtFrame;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_message::ClientProtMessageInfo;

#[client_prot(VarByte { min: 0 }, UserEvent)]
pub struct FriendSetRank {
    pub name: String,
    pub rank: i8,
}

impl ClientProtMessage for FriendSetRank {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let name = buf.gjstr(0);
        let rank = buf.g1() as i8;
        FriendSetRank { name, rank }
    }
}
