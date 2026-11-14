use crate::network::game::client::ClientProtMessage;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[client_prot(Fixed(12), UserEvent)]
pub struct ApCoordT {
    pub x: u16,
    pub z: u16,
    pub obj: TypeId,
    pub slot: u16,
    pub com: i32,
}

impl ClientProtMessage for ApCoordT {
    #[cfg(before_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2_alt1();
        let slot = buf.g2();
        let obj = TypeId::from_g2(buf.g2_alt2());
        let z = buf.g2_alt2();
        let com = buf.g4s();
        ApCoordT {
            x,
            z,
            obj,
            slot,
            com,
        }
    }

    #[cfg(since_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = TypeId::from_g2(buf.g2());
        let slot = buf.g2();
        let x = buf.g2_alt3();
        let com = buf.g4s_alt3();
        let z = buf.g2_alt3();
        ApCoordT {
            x,
            z,
            obj,
            slot,
            com,
        }
    }
}
