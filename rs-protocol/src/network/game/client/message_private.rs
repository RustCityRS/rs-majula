use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_530)]
use crate::network::game::client_prot_message::smart_end;
#[cfg(since_647)]
use crate::network::game::client_prot_message::string_end;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
#[cfg(since_647)]
use rs_ver::STRING_TERMINATOR;

#[cfg(before_647)]
#[cfg_attr(before_530, client_prot(VarByte { min: 8 }, UserEvent))]
#[cfg_attr(since_530, client_prot(VarByte { min: 9 }, UserEvent))]
pub struct MessagePrivate {
    pub user37: i64,
    #[cfg(since_530)]
    pub chars: u16,
    pub bytes: Vec<u8>,
}

#[cfg(before_530)]
impl ClientProtMessage for MessagePrivate {
    fn decode(buf: &mut Packet, len: usize) -> Self {
        let user37 = buf.g8s();
        let mut bytes = vec![0; len - buf.pos];
        buf.gdata(&mut bytes, 0, len - buf.pos);
        MessagePrivate { user37, bytes }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for MessagePrivate {
    fn validate(data: &[u8]) -> bool {
        smart_end(data, 8).is_some()
    }

    fn decode(buf: &mut Packet, len: usize) -> Self {
        let user37 = buf.g8s();
        let chars = buf.gsmart1or2() as u16;
        let bytes = buf.data[buf.pos..len].to_vec();
        buf.pos = len;
        MessagePrivate {
            user37,
            chars,
            bytes,
        }
    }
}

#[cfg(since_647)]
#[cfg_attr(before_742, client_prot(VarByte { min: 2 }, UserEvent))]
#[cfg_attr(since_742, client_prot(VarShort { min: 2 }, UserEvent))]
pub struct MessagePrivate {
    pub name: String,
    pub chars: u16,
    pub bytes: Vec<u8>,
}

#[cfg(since_647)]
impl ClientProtMessage for MessagePrivate {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0)
            .and_then(|end| smart_end(data, end))
            .is_some()
    }

    fn decode(buf: &mut Packet, len: usize) -> Self {
        let name = buf.gjstr(STRING_TERMINATOR);
        let chars = buf.gsmart1or2() as u16;
        let bytes = buf.data[buf.pos..len].to_vec();
        buf.pos = len;
        MessagePrivate { name, chars, bytes }
    }
}
