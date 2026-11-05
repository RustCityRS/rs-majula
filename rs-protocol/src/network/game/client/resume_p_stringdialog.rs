use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::{ClientProtMessageInfo, string_end};
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::STRING_TERMINATOR;

#[client_prot(VarByte { min: 1 }, UserEvent)]
pub struct ResumePStringDialog {
    pub text: String,
}

impl ClientProtMessage for ResumePStringDialog {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0) == Some(data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        ResumePStringDialog {
            text: buf.gjstr(STRING_TERMINATOR),
        }
    }
}
