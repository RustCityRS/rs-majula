use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(Fixed(4), UserEvent)]
pub struct OpPlayerT {
    pub pid: u16,
    pub com: u16,
}

#[cfg(before_647)]
impl ClientProtMessage for OpPlayerT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayerT {
            pid: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_647)]
#[client_prot(Fixed(11), UserEvent)]
pub struct OpPlayerT {
    pub pid: u16,
    pub obj: u16,
    pub slot: u16,
    pub com: i32,
    pub ctrl: bool,
}

#[cfg(since_647)]
impl ClientProtMessage for OpPlayerT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let pid = buf.g2_alt2();
        let slot = buf.g2_alt1();
        let com = buf.g4s();
        let ctrl = buf.g1_alt3() != 0;
        let obj = buf.g2_alt1();
        OpPlayerT {
            pid,
            obj,
            slot,
            com,
            ctrl,
        }
    }
}
