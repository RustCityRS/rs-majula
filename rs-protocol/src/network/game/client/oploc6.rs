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

#[cfg_attr(before_742, client_prot(Fixed(2), UserEvent))]
#[cfg_attr(since_742, client_prot(Fixed(9), UserEvent))]
#[derive(Debug)]
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
        let loc = TypeId::from_int(buf.g4_alt3());
        let ctrl = buf.g1_alt3() != 0;
        OpLoc6 { loc, x, z, ctrl }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpLoc6 {
    const PROT: ClientProt = ClientProt::OpLoc6;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt3(self.loc.g2());
    }
}

#[cfg(all(since_647, before_742))]
impl ClientProtEncode for OpLoc6 {
    const PROT: ClientProt = ClientProt::OpLoc6;

    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.loc.g2());
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpLoc6 {
    const PROT: ClientProt = ClientProt::OpLoc6;

    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.x);
        buf.p2_alt1(self.z);
        buf.p4_alt3(self.loc.int());
        buf.p1_alt3(self.ctrl as u8);
    }
}
