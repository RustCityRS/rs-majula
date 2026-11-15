use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[cfg_attr(before_647, client_prot(Fixed(6), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(7), UserEvent))]
pub struct OpObj2 {
    pub x: u16,
    pub z: u16,
    pub obj: TypeId,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(before_530)]
impl ClientProtMessage for OpObj2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpObj2 {
            x: buf.g2(),
            z: buf.g2(),
            obj: TypeId::from_g2(buf.g2()),
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpObj2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let z = buf.g2_alt1();
        let x = buf.g2();
        let obj = TypeId::from_g2(buf.g2_alt3());
        OpObj2 { x, z, obj }
    }
}

#[cfg(since_647)]
#[cfg(before_742)]
impl ClientProtMessage for OpObj2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let z = buf.g2_alt1();
        let ctrl = buf.g1_alt1() != 0;
        let x = buf.g2_alt2();
        let obj = TypeId::from_g2(buf.g2_alt1());
        OpObj2 { obj, x, z, ctrl }
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpObj2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let ctrl = buf.g1() != 0;
        let x = buf.g2_alt1();
        let obj = TypeId::from_g2(buf.g2_alt2());
        let z = buf.g2_alt3();
        OpObj2 { obj, x, z, ctrl }
    }
}
