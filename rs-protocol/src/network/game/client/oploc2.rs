use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg_attr(before_647, client_prot(Fixed(6), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(7), UserEvent))]
pub struct OpLoc2 {
    pub x: u16,
    pub z: u16,
    pub loc: u16,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(before_647)]
impl ClientProtMessage for OpLoc2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpLoc2 {
            x: buf.g2(),
            z: buf.g2(),
            loc: buf.g2(),
        }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for OpLoc2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let loc = buf.ig2();
        let ctrl = buf.g1_alt1() != 0;
        let z = buf.g2_alt1();
        let x = buf.ig2_alt1();
        OpLoc2 { loc, x, z, ctrl }
    }
}
