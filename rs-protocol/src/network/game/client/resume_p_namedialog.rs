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

#[cfg(all(since_530, before_647))]
#[client_prot(Fixed(8), UserEvent)]
pub struct ResumePNameDialog {
    pub user37: i64,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for ResumePNameDialog {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ResumePNameDialog { user37: buf.g8s() }
    }
}

#[cfg(since_647)]
#[client_prot(VarByte { min: 1 }, UserEvent)]
pub struct ResumePNameDialog {
    pub name: String,
}

#[cfg(since_647)]
impl ClientProtMessage for ResumePNameDialog {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0) == Some(data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        ResumePNameDialog {
            name: buf.gjstr(STRING_TERMINATOR),
        }
    }
}
