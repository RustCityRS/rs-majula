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

#[cfg_attr(before_647, client_prot(Fixed(6), UserEvent))]
#[cfg_attr(all(since_647, before_742), client_prot(Fixed(7), UserEvent))]
#[cfg_attr(since_742, client_prot(Fixed(9), UserEvent))]
#[derive(Debug)]
pub struct OpLoc2 {
    pub x: u16,
    pub z: u16,
    pub loc: TypeId,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(before_530)]
impl ClientProtMessage for OpLoc2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpLoc2 {
            x: buf.g2(),
            z: buf.g2(),
            loc: TypeId::from_g2(buf.g2()),
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpLoc2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let z = buf.g2_alt3();
        let x = buf.g2_alt1();
        let loc = TypeId::from_g2(buf.g2());
        OpLoc2 { x, z, loc }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpLoc2 {
    const PROT: ClientProt = ClientProt::OpLoc2;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt3(self.z);
        buf.p2_alt1(self.x);
        buf.p2(self.loc.g2());
    }
}

#[cfg(all(since_647, before_742))]
impl ClientProtMessage for OpLoc2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let loc = TypeId::from_g2(buf.g2_alt1());
        let ctrl = buf.g1_alt2() != 0;
        let z = buf.g2_alt2();
        let x = buf.g2_alt3();
        OpLoc2 { loc, x, z, ctrl }
    }
}

#[cfg(all(since_647, before_742))]
impl ClientProtEncode for OpLoc2 {
    const PROT: ClientProt = ClientProt::OpLoc2;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.loc.g2());
        buf.p1_alt2(self.ctrl as u8);
        buf.p2_alt2(self.z);
        buf.p2_alt3(self.x);
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpLoc2 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2();
        let z = buf.g2_alt1();
        let loc = TypeId::from_int(buf.g4_alt3());
        let ctrl = buf.g1_alt3() != 0;
        OpLoc2 { loc, x, z, ctrl }
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpLoc2 {
    const PROT: ClientProt = ClientProt::OpLoc2;

    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.x);
        buf.p2_alt1(self.z);
        buf.p4_alt3(self.loc.int());
        buf.p1_alt3(self.ctrl as u8);
    }
}
