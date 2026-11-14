use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[client_prot(Fixed(16), UserEvent)]
pub struct IfButtonD {
    pub com: i32,
    pub slot: u16,
    pub obj: TypeId,
    pub com2: i32,
    pub slot2: u16,
    pub obj2: TypeId,
}

impl ClientProtMessage for IfButtonD {
    #[cfg(before_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4s();
        let slot = buf.g2_alt3();
        let slot2 = buf.g2_alt3();
        let com2 = buf.g4s_alt2();
        let obj = TypeId::from_g2(buf.g2_alt1());
        let obj2 = TypeId::from_g2(buf.g2());
        IfButtonD {
            com,
            slot,
            obj,
            com2,
            slot2,
            obj2,
        }
    }

    #[cfg(since_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com2 = buf.g4s_alt1();
        let obj = TypeId::from_g2(buf.g2_alt2());
        let com = buf.g4s();
        let slot2 = buf.g2_alt1();
        let slot = buf.g2_alt2();
        let obj2 = TypeId::from_g2(buf.g2_alt3());
        IfButtonD {
            com,
            slot,
            obj,
            com2,
            slot2,
            obj2,
        }
    }
}
