use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg_attr(before_647, client_prot(Fixed(6), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(7), UserEvent))]
pub struct OpObj1 {
    pub x: u16,
    pub z: u16,
    pub obj: u16,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(before_647)]
impl ClientProtMessage for OpObj1 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpObj1 {
            x: buf.g2(),
            z: buf.g2(),
            obj: buf.g2(),
        }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for OpObj1 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let ctrl = buf.g1_alt2() != 0;
        let obj = buf.g2_alt1();
        let x = buf.ig2_alt1();
        let z = buf.ig2();
        OpObj1 { obj, x, z, ctrl }
    }
}
