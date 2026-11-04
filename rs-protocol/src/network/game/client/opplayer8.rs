use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[client_prot(Fixed(3), UserEvent)]
pub struct OpPlayer8 {
    pub pid: u16,
    pub ctrl: bool,
}

impl ClientProtMessage for OpPlayer8 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let (ctrl, pid) = (buf.g1() != 0, buf.ig2());
        OpPlayer8 { pid, ctrl }
    }
}
