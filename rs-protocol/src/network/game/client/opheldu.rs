#[cfg(all(since_530, before_647))]
use crate::network::game::client::ClientProtEncode;
use crate::network::game::client::ClientProtMessage;
#[cfg(all(since_530, before_647))]
use crate::network::game::client_prot::ClientProt;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[cfg_attr(before_530, client_prot(Fixed(12), UserEvent))]
#[cfg_attr(since_530, client_prot(Fixed(16), UserEvent))]
#[derive(Debug)]
pub struct OpHeldU {
    pub obj: TypeId,
    pub slot: u16,
    #[cfg(before_530)]
    pub com: u16,
    #[cfg(since_530)]
    pub com: i32,
    pub obj2: TypeId,
    pub slot2: u16,
    #[cfg(before_530)]
    pub com2: u16,
    #[cfg(since_530)]
    pub com2: i32,
}

#[cfg(before_530)]
impl ClientProtMessage for OpHeldU {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpHeldU {
            obj: TypeId::from_g2(buf.g2()),
            slot: buf.g2(),
            com: buf.g2(),
            obj2: TypeId::from_g2(buf.g2()),
            slot2: buf.g2(),
            com2: buf.g2(),
        }
    }
}

#[cfg(since_530)]
impl ClientProtMessage for OpHeldU {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let slot2 = buf.g2();
        let com = buf.g4_alt1();
        let slot = buf.g2_alt1();
        let com2 = buf.g4_alt1();
        let obj2 = TypeId::from_g2(buf.g2_alt3());
        let obj = TypeId::from_g2(buf.g2_alt3());
        OpHeldU {
            obj,
            slot,
            com,
            obj2,
            slot2,
            com2,
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpHeldU {
    const PROT: ClientProt = ClientProt::OpHeldU;

    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.slot2);
        buf.p4_alt1(self.com);
        buf.p2_alt1(self.slot);
        buf.p4_alt1(self.com2);
        buf.p2_alt3(self.obj2.g2());
        buf.p2_alt3(self.obj.g2());
    }
}
