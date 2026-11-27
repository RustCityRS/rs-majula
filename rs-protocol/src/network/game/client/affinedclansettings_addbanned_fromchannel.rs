use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::{ClientProtMessageInfo, string_end};
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::STRING_TERMINATOR;

#[client_prot(VarByte { min: 3 }, UserEvent)]
pub struct AffinedClansettingsAddBannedFromChannel {
    pub index: u16,
    pub name: String,
}

impl ClientProtMessage for AffinedClansettingsAddBannedFromChannel {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 2) == Some(data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        AffinedClansettingsAddBannedFromChannel {
            index: buf.g2(),
            name: buf.gjstr(STRING_TERMINATOR),
        }
    }
}
