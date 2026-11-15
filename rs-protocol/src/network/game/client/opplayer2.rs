use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg_attr(before_647, client_prot(Fixed(2), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(3), UserEvent))]
pub struct OpPlayer2 {
    pub pid: u16,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(before_647)]
impl ClientProtMessage for OpPlayer2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayer2 { pid: buf.g2() }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for OpPlayer2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let (ctrl, pid) = (buf.g1_alt2() != 0, buf.g2());
        #[cfg(since_742)]
        let (ctrl, pid) = (buf.g1() != 0, buf.g2_alt1());
        OpPlayer2 { pid, ctrl }
    }
}
