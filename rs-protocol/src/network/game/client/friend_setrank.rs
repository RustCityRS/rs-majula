use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::{ClientProtMessageInfo, string_end};
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::STRING_TERMINATOR;

#[client_prot(VarByte { min: 2 }, UserEvent)]
pub struct FriendSetRank {
    pub rank: i8,
    pub name: String,
}

impl ClientProtMessage for FriendSetRank {
    #[cfg(before_742)]
    fn validate(data: &[u8]) -> bool {
        string_end(data, 1) == Some(data.len())
    }

    #[cfg(since_742)]
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0).is_some_and(|end| end + 1 == data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let rank = buf.g1_alt1() as i8;
        let name = buf.gjstr(STRING_TERMINATOR);
        #[cfg(since_742)]
        let rank = buf.g1_alt1() as i8;
        FriendSetRank { rank, name }
    }
}
