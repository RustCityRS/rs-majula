use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(Fixed(8), UserEvent)]
pub struct OpLocT {
    pub x: u16,
    pub z: u16,
    pub loc: u16,
    pub com: u16,
}

#[cfg(before_647)]
impl ClientProtMessage for OpLocT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpLocT {
            x: buf.g2(),
            z: buf.g2(),
            loc: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_647)]
#[client_prot(Fixed(15), UserEvent)]
pub struct OpLocT {
    pub loc: u16,
    pub x: u16,
    pub z: u16,
    pub obj: u16,
    pub slot: u16,
    pub com: i32,
    pub ctrl: bool,
}

#[cfg(since_647)]
impl ClientProtMessage for OpLocT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = buf.g2_alt2();
        let slot = buf.g2_alt2();
        let z = buf.g2();
        let com = buf.g4s_alt2();
        let x = buf.g2_alt2();
        let ctrl = buf.g1_alt1() != 0;
        let loc = buf.g2_alt1();
        OpLocT {
            loc,
            x,
            z,
            obj,
            slot,
            com,
            ctrl,
        }
    }
}
