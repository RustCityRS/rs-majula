#[cfg(since_530)]
use crate::network::game::client::ClientProtEncode;
use crate::network::game::client::ClientProtMessage;
#[cfg(since_530)]
use crate::network::game::client_prot::ClientProt;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::{ClientProtMessageInfo, string_end};
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::STRING_TERMINATOR;

#[cfg_attr(before_742, client_prot(VarByte { min: 3 }, UserEvent))]
#[cfg_attr(since_742, client_prot(VarShort { min: 4 }, UserEvent))]
#[derive(Debug)]
pub struct UrlRequest {
    pub url: String,
    pub text: String,
    #[cfg(since_742)]
    pub extra: String,
    pub mode: u8,
}

impl ClientProtMessage for UrlRequest {
    fn validate(data: &[u8]) -> bool {
        let end = string_end(data, 0).and_then(|end| string_end(data, end));
        #[cfg(since_742)]
        let end = end.and_then(|end| string_end(data, end));
        end.is_some_and(|end| end + 1 == data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        UrlRequest {
            url: buf.gjstr(STRING_TERMINATOR),
            text: buf.gjstr(STRING_TERMINATOR),
            #[cfg(since_742)]
            extra: buf.gjstr(STRING_TERMINATOR),
            mode: buf.g1(),
        }
    }
}

#[cfg(since_530)]
impl ClientProtEncode for UrlRequest {
    const PROT: ClientProt = ClientProt::UrlRequest;

    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.url, STRING_TERMINATOR);
        buf.pjstr(&self.text, STRING_TERMINATOR);
        #[cfg(since_742)]
        buf.pjstr(&self.extra, STRING_TERMINATOR);
        buf.p1(self.mode);
    }
}
