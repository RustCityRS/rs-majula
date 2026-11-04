use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::{ClientProtMessageInfo, string_end};
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(VarByte { min: 1 }, UserEvent)]
pub struct ResumePClanforumqfcDialog {
    pub text: String,
}

impl ClientProtMessage for ResumePClanforumqfcDialog {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0) == Some(data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        ResumePClanforumqfcDialog { text: buf.gjstr(0) }
    }
}
