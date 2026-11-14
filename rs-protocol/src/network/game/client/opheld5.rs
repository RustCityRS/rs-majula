use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[cfg_attr(before_530, client_prot(Fixed(6), UserEvent))]
#[cfg_attr(since_530, client_prot(Fixed(8), UserEvent))]
pub struct OpHeld5 {
    pub obj: TypeId,
    pub slot: u16,
    #[cfg(before_530)]
    pub com: u16,
    #[cfg(since_530)]
    pub com: i32,
}

#[cfg(before_530)]
impl ClientProtMessage for OpHeld5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpHeld5 {
            obj: TypeId::from_g2(buf.g2()),
            slot: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_530)]
impl ClientProtMessage for OpHeld5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = TypeId::from_g2(buf.g2_alt2());
        let slot = buf.g2_alt2();
        let com = buf.g4s_alt3();
        OpHeld5 { obj, slot, com }
    }
}
