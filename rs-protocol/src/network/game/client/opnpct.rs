use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(Fixed(4), UserEvent)]
pub struct OpNpcT {
    pub nid: u16,
    pub com: u16,
}

#[cfg(before_647)]
impl ClientProtMessage for OpNpcT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpNpcT {
            nid: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_647)]
#[client_prot(Fixed(11), UserEvent)]
pub struct OpNpcT {
    pub nid: u16,
    pub obj: u16,
    pub slot: u16,
    pub com: u32,
    pub ctrl: bool,
}

#[cfg(since_647)]
impl ClientProtMessage for OpNpcT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = buf.g2_alt1();
        let slot = buf.g2_alt1();
        let nid = buf.ig2();
        let com = buf.g4_alt1() as u32;
        let ctrl = buf.g1_alt1() != 0;
        OpNpcT {
            nid,
            obj,
            slot,
            com,
            ctrl,
        }
    }
}
