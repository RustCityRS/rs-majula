use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
#[cfg(since_647)]
use rs_ver::TypeId;

#[cfg(before_647)]
#[client_prot(Fixed(4), UserEvent)]
pub struct OpPlayerT {
    pub pid: u16,
    pub com: u16,
}

#[cfg(before_647)]
impl ClientProtMessage for OpPlayerT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayerT {
            pid: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_647)]
#[client_prot(Fixed(11), UserEvent)]
pub struct OpPlayerT {
    pub pid: u16,
    pub obj: TypeId,
    pub slot: u16,
    pub com: i32,
    pub ctrl: bool,
}

#[cfg(since_647)]
impl ClientProtMessage for OpPlayerT {
    #[cfg(before_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let pid = buf.g2_alt2();
        let slot = buf.g2_alt1();
        let com = buf.g4s();
        let ctrl = buf.g1_alt3() != 0;
        let obj = TypeId::from_g2(buf.g2_alt1());
        OpPlayerT {
            pid,
            obj,
            slot,
            com,
            ctrl,
        }
    }

    #[cfg(since_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = TypeId::from_g2(buf.g2_alt1());
        let ctrl = buf.g1_alt3() != 0;
        let pid = buf.g2_alt3();
        let com = buf.g4s_alt2();
        let slot = buf.g2_alt1();
        OpPlayerT {
            pid,
            obj,
            slot,
            com,
            ctrl,
        }
    }
}
