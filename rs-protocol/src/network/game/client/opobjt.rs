use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg(before_647)]
#[client_prot(Fixed(8), UserEvent)]
pub struct OpObjT {
    pub x: u16,
    pub z: u16,
    pub obj: u16,
    pub com: u16,
}

#[cfg(before_647)]
impl ClientProtMessage for OpObjT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpObjT {
            x: buf.g2(),
            z: buf.g2(),
            obj: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_647)]
#[client_prot(Fixed(15), UserEvent)]
pub struct OpObjT {
    pub obj: u16,
    pub x: u16,
    pub z: u16,
    pub obj2: u16,
    pub slot: u16,
    pub com: u32,
    pub ctrl: bool,
}

#[cfg(since_647)]
impl ClientProtMessage for OpObjT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj2 = buf.ig2();
        let obj = buf.ig2();
        let slot = buf.g2();
        let com = buf.g4_alt2() as u32;
        let ctrl = buf.g1_alt3() != 0;
        let z = buf.g2_alt1();
        let x = buf.g2_alt1();
        OpObjT {
            obj,
            x,
            z,
            obj2,
            slot,
            com,
            ctrl,
        }
    }
}
