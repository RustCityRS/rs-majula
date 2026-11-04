use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_647)]
use crate::network::game::client_prot_message::string_end;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(Fixed(8), UserEvent)]
pub struct IgnoreListAdd {
    pub user37: i64,
}

#[cfg(before_647)]
impl ClientProtMessage for IgnoreListAdd {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        IgnoreListAdd { user37: buf.g8s() }
    }
}

#[cfg(since_647)]
#[client_prot(VarByte { min: 2 }, UserEvent)]
pub struct IgnoreListAdd {
    pub name: String,
    pub temporary: bool,
}

#[cfg(since_647)]
impl ClientProtMessage for IgnoreListAdd {
    fn validate(data: &[u8]) -> bool {
        string_end(data, 0).is_some_and(|end| end + 1 == data.len())
    }

    fn decode(buf: &mut Packet, _: usize) -> Self {
        let name = buf.gjstr(0);
        let temporary = buf.g1() == 1;
        IgnoreListAdd { name, temporary }
    }
}
