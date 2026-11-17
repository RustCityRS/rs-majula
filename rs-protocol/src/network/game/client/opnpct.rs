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
#[cfg(since_647)]
use rs_ver::TypeId;

#[cfg(before_530)]
#[client_prot(Fixed(4), UserEvent)]
#[derive(Debug)]
pub struct OpNpcT {
    pub nid: u16,
    pub com: u16,
}

#[cfg(before_530)]
impl ClientProtMessage for OpNpcT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpNpcT {
            nid: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_530)]
#[cfg_attr(before_647, client_prot(Fixed(8), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(11), UserEvent))]
#[derive(Debug)]
pub struct OpNpcT {
    pub nid: u16,
    #[cfg(since_647)]
    pub obj: TypeId,
    pub slot: u16,
    pub com: i32,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpNpcT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4_alt1();
        let slot = buf.g2_alt2();
        let nid = buf.g2_alt3();
        OpNpcT { nid, slot, com }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpNpcT {
    const PROT: ClientProt = ClientProt::OpNpcT;

    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt1(self.com);
        buf.p2_alt2(self.slot);
        buf.p2_alt3(self.nid);
    }
}

#[cfg(since_647)]
#[cfg(before_742)]
impl ClientProtMessage for OpNpcT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = TypeId::from_g2(buf.g2_alt2());
        let slot = buf.g2_alt2();
        let nid = buf.g2_alt1();
        let com = buf.g4_alt3();
        let ctrl = buf.g1_alt2() != 0;
        OpNpcT {
            nid,
            obj,
            slot,
            com,
            ctrl,
        }
    }
}

#[cfg(since_647)]
#[cfg(before_742)]
impl ClientProtEncode for OpNpcT {
    const PROT: ClientProt = ClientProt::OpNpcT;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt2(self.obj.g2());
        buf.p2_alt2(self.slot);
        buf.p2_alt1(self.nid);
        buf.p4_alt3(self.com);
        buf.p1_alt2(self.ctrl as u8);
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpNpcT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let ctrl = buf.g1_alt3() != 0;
        let nid = buf.g2();
        let com = buf.g4s();
        let obj = TypeId::from_g2(buf.g2_alt3());
        let slot = buf.g2_alt1();
        OpNpcT {
            nid,
            obj,
            slot,
            com,
            ctrl,
        }
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpNpcT {
    const PROT: ClientProt = ClientProt::OpNpcT;

    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt3(self.ctrl as u8);
        buf.p2(self.nid);
        buf.p4(self.com);
        buf.p2_alt3(self.obj.g2());
        buf.p2_alt1(self.slot);
    }
}
