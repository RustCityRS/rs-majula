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
#[cfg_attr(since_530, client_prot(Fixed(14), UserEvent))]
#[derive(Debug)]
pub struct OpObjU {
    pub x: u16,
    pub z: u16,
    pub obj: TypeId,
    pub use_obj: TypeId,
    pub slot: u16,
    #[cfg(before_530)]
    pub com: u16,
    #[cfg(since_530)]
    pub com: i32,
}

#[cfg(before_530)]
impl ClientProtMessage for OpObjU {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpObjU {
            x: buf.g2(),
            z: buf.g2(),
            obj: TypeId::from_g2(buf.g2()),
            use_obj: TypeId::from_g2(buf.g2()),
            slot: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_530)]
impl ClientProtMessage for OpObjU {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2_alt3();
        let slot = buf.g2_alt1();
        let use_obj = TypeId::from_g2(buf.g2_alt1());
        let obj = TypeId::from_g2(buf.g2_alt1());
        let z = buf.g2_alt3();
        let com = buf.g4_alt3();
        OpObjU {
            x,
            z,
            obj,
            use_obj,
            slot,
            com,
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpObjU {
    const PROT: ClientProt = ClientProt::OpObjU;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt3(self.x);
        buf.p2_alt1(self.slot);
        buf.p2_alt1(self.use_obj.g2());
        buf.p2_alt1(self.obj.g2());
        buf.p2_alt3(self.z);
        buf.p4_alt3(self.com);
    }
}
