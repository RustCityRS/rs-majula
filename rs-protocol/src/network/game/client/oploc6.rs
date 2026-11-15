use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[cfg_attr(before_742, client_prot(Fixed(2), UserEvent))]
#[cfg_attr(since_742, client_prot(Fixed(9), UserEvent))]
pub struct OpLoc6 {
    #[cfg(since_742)]
    pub x: u16,
    #[cfg(since_742)]
    pub z: u16,
    pub loc: TypeId,
    #[cfg(since_742)]
    pub ctrl: bool,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpLoc6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpLoc6 {
            loc: TypeId::from_g2(buf.g2_alt3()),
        }
    }
}

#[cfg(all(since_647, before_742))]
impl ClientProtMessage for OpLoc6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpLoc6 {
            loc: TypeId::from_g2(buf.g2()),
        }
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpLoc6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2();
        let z = buf.g2_alt1();
        let loc = TypeId::from_int(buf.g4s_alt3());
        let ctrl = buf.g1_alt3() != 0;
        OpLoc6 { loc, x, z, ctrl }
    }
}
