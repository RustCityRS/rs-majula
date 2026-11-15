use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
#[cfg(since_647)]
use rs_ver::TypeId;

#[cfg_attr(before_647, client_prot(Fixed(6), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(8), UserEvent))]
pub struct IfButton6 {
    pub com: i32,
    pub slot: u16,
    #[cfg(since_647)]
    pub obj: TypeId,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for IfButton6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4s();
        let slot = buf.g2();
        IfButton6 { com, slot }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for IfButton6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let (com, obj, slot) = (buf.g4s(), buf.g2_alt3(), buf.g2_alt1());
        #[cfg(since_742)]
        let (obj, slot, com) = (buf.g2(), buf.g2_alt3(), buf.g4s_alt2());
        IfButton6 {
            com,
            slot,
            obj: TypeId::from_g2(obj),
        }
    }
}
