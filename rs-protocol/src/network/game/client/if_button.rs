use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg_attr(before_530, client_prot(Fixed(2), UserEvent))]
#[cfg_attr(since_530, client_prot(Fixed(4), UserEvent))]
pub struct IfButton {
    #[cfg(before_530)]
    pub com: u16,
    #[cfg(since_530)]
    pub com: i32,
}

#[cfg(before_530)]
impl ClientProtMessage for IfButton {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        IfButton { com: buf.g2() }
    }
}

#[cfg(since_530)]
impl ClientProtMessage for IfButton {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        IfButton { com: buf.g4s() }
    }
}
