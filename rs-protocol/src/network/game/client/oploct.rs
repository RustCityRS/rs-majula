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
pub struct OpLocT {
    pub x: u16,
    pub z: u16,
    pub loc: TypeId,
    pub com: u16,
}

#[cfg(before_530)]
impl ClientProtMessage for OpLocT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpLocT {
            x: buf.g2(),
            z: buf.g2(),
            loc: TypeId::from_g2(buf.g2()),
            com: buf.g2(),
        }
    }
}

#[cfg(since_530)]
#[cfg_attr(before_647, client_prot(Fixed(12), UserEvent))]
#[cfg_attr(all(since_647, before_742), client_prot(Fixed(15), UserEvent))]
#[cfg_attr(since_742, client_prot(Fixed(17), UserEvent))]
#[derive(Debug)]
pub struct OpLocT {
    pub loc: TypeId,
    pub x: u16,
    pub z: u16,
    #[cfg(since_647)]
    pub obj: TypeId,
    pub slot: u16,
    pub com: i32,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpLocT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let z = buf.g2_alt3();
        let x = buf.g2_alt2();
        let slot = buf.g2_alt3();
        let com = buf.g4_alt2();
        let loc = TypeId::from_g2(buf.g2_alt2());
        OpLocT {
            loc,
            x,
            z,
            slot,
            com,
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpLocT {
    const PROT: ClientProt = ClientProt::OpLocT;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt3(self.z);
        buf.p2_alt2(self.x);
        buf.p2_alt3(self.slot);
        buf.p4_alt2(self.com);
        buf.p2_alt2(self.loc.g2());
    }
}

#[cfg(all(since_647, before_742))]
impl ClientProtMessage for OpLocT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = TypeId::from_g2(buf.g2_alt2());
        let slot = buf.g2_alt2();
        let z = buf.g2();
        let com = buf.g4_alt2();
        let x = buf.g2_alt2();
        let ctrl = buf.g1_alt1() != 0;
        let loc = TypeId::from_g2(buf.g2_alt1());
        OpLocT {
            loc,
            x,
            z,
            obj,
            slot,
            com,
            ctrl,
        }
    }
}

#[cfg(all(since_647, before_742))]
impl ClientProtEncode for OpLocT {
    const PROT: ClientProt = ClientProt::OpLocT;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt2(self.obj.g2());
        buf.p2_alt2(self.slot);
        buf.p2(self.z);
        buf.p4_alt2(self.com);
        buf.p2_alt2(self.x);
        buf.p1_alt1(self.ctrl as u8);
        buf.p2_alt1(self.loc.g2());
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpLocT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4_alt2();
        let ctrl = buf.g1() != 0;
        let x = buf.g2_alt2();
        let z = buf.g2_alt2();
        let slot = buf.g2();
        let loc = TypeId::from_int(buf.g4s());
        let obj = TypeId::from_g2(buf.g2_alt2());
        OpLocT {
            loc,
            x,
            z,
            obj,
            slot,
            com,
            ctrl,
        }
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpLocT {
    const PROT: ClientProt = ClientProt::OpLocT;

    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.com);
        buf.p1(self.ctrl as u8);
        buf.p2_alt2(self.x);
        buf.p2_alt2(self.z);
        buf.p2(self.slot);
        buf.p4(self.loc.int());
        buf.p2_alt2(self.obj.g2());
    }
}
