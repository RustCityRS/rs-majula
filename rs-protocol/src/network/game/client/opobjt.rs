#[cfg(since_530)]
use crate::network::game::client::ClientProtEncode;
use crate::network::game::client::ClientProtMessage;
#[cfg(since_530)]
use crate::network::game::client_prot::ClientProt;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;
use rs_ver::TypeId;

#[cfg(before_530)]
#[client_prot(Fixed(8), UserEvent)]
#[derive(Debug)]
pub struct OpObjT {
    pub x: u16,
    pub z: u16,
    pub obj: TypeId,
    pub com: u16,
}

#[cfg(before_530)]
impl ClientProtMessage for OpObjT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpObjT {
            x: buf.g2(),
            z: buf.g2(),
            obj: TypeId::from_g2(buf.g2()),
            com: buf.g2(),
        }
    }
}

#[cfg(since_530)]
#[cfg_attr(before_647, client_prot(Fixed(12), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(15), UserEvent))]
#[derive(Debug)]
pub struct OpObjT {
    pub obj: TypeId,
    pub x: u16,
    pub z: u16,
    #[cfg(since_647)]
    pub obj2: TypeId,
    pub slot: u16,
    pub com: i32,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpObjT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4_alt2();
        let z = buf.g2();
        let obj = TypeId::from_g2(buf.g2_alt3());
        let x = buf.g2_alt3();
        let slot = buf.g2_alt1();
        OpObjT {
            obj,
            x,
            z,
            slot,
            com,
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpObjT {
    const PROT: ClientProt = ClientProt::OpObjT;

    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.com);
        buf.p2(self.z);
        buf.p2_alt3(self.obj.g2());
        buf.p2_alt3(self.x);
        buf.p2_alt1(self.slot);
    }
}

#[cfg(since_647)]
#[cfg(before_742)]
impl ClientProtMessage for OpObjT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj2 = TypeId::from_g2(buf.g2_alt1());
        let obj = TypeId::from_g2(buf.g2_alt1());
        let slot = buf.g2();
        let com = buf.g4_alt2();
        let ctrl = buf.g1_alt1() != 0;
        let z = buf.g2_alt2();
        let x = buf.g2_alt2();
        OpObjT {
            obj,
            x,
            z,
            obj2,
            slot,
            com,
            ctrl,
        }
    }
}

#[cfg(since_647)]
#[cfg(before_742)]
impl ClientProtEncode for OpObjT {
    const PROT: ClientProt = ClientProt::OpObjT;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.obj2.g2());
        buf.p2_alt1(self.obj.g2());
        buf.p2(self.slot);
        buf.p4_alt2(self.com);
        buf.p1_alt1(self.ctrl as u8);
        buf.p2_alt2(self.z);
        buf.p2_alt2(self.x);
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpObjT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = TypeId::from_g2(buf.g2_alt1());
        let slot = buf.g2_alt3();
        let z = buf.g2_alt2();
        let x = buf.g2_alt3();
        let obj2 = TypeId::from_g2(buf.g2_alt1());
        let ctrl = buf.g1_alt3() != 0;
        let com = buf.g4_alt2();
        OpObjT {
            obj,
            x,
            z,
            obj2,
            slot,
            com,
            ctrl,
        }
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpObjT {
    const PROT: ClientProt = ClientProt::OpObjT;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.obj.g2());
        buf.p2_alt3(self.slot);
        buf.p2_alt2(self.z);
        buf.p2_alt3(self.x);
        buf.p2_alt1(self.obj2.g2());
        buf.p1_alt3(self.ctrl as u8);
        buf.p4_alt2(self.com);
    }
}
