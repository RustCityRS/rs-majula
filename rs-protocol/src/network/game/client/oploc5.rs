use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[cfg_attr(before_647, client_prot(Fixed(6), UserEvent))]
#[cfg_attr(all(since_647, before_742), client_prot(Fixed(7), UserEvent))]
#[cfg_attr(since_742, client_prot(Fixed(9), UserEvent))]
pub struct OpLoc5 {
    pub x: u16,
    pub z: u16,
    pub loc: TypeId,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(before_530)]
impl ClientProtMessage for OpLoc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpLoc5 {
            x: buf.g2(),
            z: buf.g2(),
            loc: TypeId::from_g2(buf.g2()),
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpLoc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let loc = TypeId::from_g2(buf.g2_alt3());
        let x = buf.g2_alt3();
        let z = buf.g2_alt3();
        OpLoc5 { x, z, loc }
    }
}

#[cfg(all(since_647, before_742))]
impl ClientProtMessage for OpLoc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let loc = TypeId::from_g2(buf.g2_alt2());
        let x = buf.g2_alt2();
        let ctrl = buf.g1_alt1() != 0;
        let z = buf.g2_alt1();
        OpLoc5 { loc, x, z, ctrl }
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpLoc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2();
        let z = buf.g2_alt1();
        let loc = TypeId::from_int(buf.g4_alt3());
        let ctrl = buf.g1_alt3() != 0;
        OpLoc5 { loc, x, z, ctrl }
    }
}
