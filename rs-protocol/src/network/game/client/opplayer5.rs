#[cfg(since_254)]
use crate::network::game::client::ClientProtMessage;
#[cfg(since_254)]
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
#[cfg(since_254)]
use crate::network::game::client_prot_message::ClientProtMessageInfo;
#[cfg(since_254)]
use rs_io::Packet;
#[cfg(since_254)]
use rs_protocol_macros::client_prot;

#[cfg(since_254)]
#[cfg_attr(before_647, client_prot(Fixed(2), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(3), UserEvent))]
pub struct OpPlayer5 {
    pub pid: u16,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(since_254)]
#[cfg(before_647)]
impl ClientProtMessage for OpPlayer5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayer5 { pid: buf.g2() }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for OpPlayer5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let (ctrl, pid) = (buf.g1_alt3() != 0, buf.ig2());
        OpPlayer5 { pid, ctrl }
    }
}
