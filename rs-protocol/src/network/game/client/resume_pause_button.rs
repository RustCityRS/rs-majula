use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(Fixed(2), UserEvent)]
pub struct ResumePauseButton {
    pub com: u16,
}

#[cfg(before_647)]
impl ClientProtMessage for ResumePauseButton {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ResumePauseButton { com: buf.g2() }
    }
}

#[cfg(since_647)]
#[client_prot(Fixed(6), UserEvent)]
pub struct ResumePauseButton {
    pub com: u32,
    pub slot: u16,
}

#[cfg(since_647)]
impl ClientProtMessage for ResumePauseButton {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let slot = buf.g2_alt1();
        let com = buf.g4s_alt2() as u32;
        ResumePauseButton { com, slot }
    }
}
