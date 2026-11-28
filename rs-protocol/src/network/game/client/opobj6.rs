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
#[cfg_attr(since_742, client_prot(Fixed(7), UserEvent))]
#[derive(Debug)]
pub struct OpObj6 {
    #[cfg(since_742)]
    pub x: u16,
    #[cfg(since_742)]
    pub z: u16,
    pub obj: TypeId,
    #[cfg(since_742)]
    pub ctrl: bool,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpObj6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpObj6 {
            obj: TypeId::from_g2(buf.g2_alt3()),
        }
    }
}

#[cfg(all(since_647, before_742))]
impl ClientProtMessage for OpObj6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpObj6 {
            obj: TypeId::from_g2(buf.g2()),
        }
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpObj6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let ctrl = buf.g1() != 0;
        let x = buf.g2_alt1();
        let obj = TypeId::from_g2(buf.g2_alt2());
        let z = buf.g2_alt3();
        OpObj6 { obj, x, z, ctrl }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpObj6 {
    const PROT: ClientProt = ClientProt::OpObj6;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt3(self.obj.g2());
    }
}

#[cfg(all(since_647, before_742))]
impl ClientProtEncode for OpObj6 {
    const PROT: ClientProt = ClientProt::OpObj6;

    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.obj.g2());
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpObj6 {
    const PROT: ClientProt = ClientProt::OpObj6;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.ctrl as u8);
        buf.p2_alt1(self.x);
        buf.p2_alt2(self.obj.g2());
        buf.p2_alt3(self.z);
    }
}
