use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg_attr(before_647, client_prot(Fixed(2), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(3), UserEvent))]
pub struct OpPlayer4 {
    pub pid: u16,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(before_647)]
impl ClientProtMessage for OpPlayer4 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayer4 { pid: buf.g2() }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for OpPlayer4 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let pid = buf.g2_alt1();
        let ctrl = buf.g1() != 0;
        OpPlayer4 { pid, ctrl }
    }
}
