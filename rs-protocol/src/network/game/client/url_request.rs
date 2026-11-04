use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::{ClientProtMessageInfo, string_end};
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(VarByte { min: 3 }, ClientEvent)]
pub struct UrlRequest {
    pub url: String,
    pub text: String,
    pub mode: u8,
}

impl ClientProtMessage for UrlRequest {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0)
            .and_then(|end| string_end(data, end))
            .is_some_and(|end| end + 1 == data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        UrlRequest {
            url: buf.gjstr(0),
            text: buf.gjstr(0),
            mode: buf.g1(),
        }
    }
}
