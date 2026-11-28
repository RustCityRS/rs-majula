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
#[cfg_attr(since_647, client_prot(Fixed(7), UserEvent))]
#[derive(Debug)]
pub struct OpObj3 {
    pub x: u16,
    pub z: u16,
    pub obj: TypeId,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(before_530)]
impl ClientProtMessage for OpObj3 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpObj3 {
            x: buf.g2(),
            z: buf.g2(),
            obj: TypeId::from_g2(buf.g2()),
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpObj3 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2_alt1();
        let obj = TypeId::from_g2(buf.g2());
        let z = buf.g2_alt3();
        OpObj3 { x, z, obj }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpObj3 {
    const PROT: ClientProt = ClientProt::OpObj3;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.x);
        buf.p2(self.obj.g2());
        buf.p2_alt3(self.z);
    }
}

#[cfg(since_647)]
#[cfg(before_742)]
impl ClientProtMessage for OpObj3 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = TypeId::from_g2(buf.g2_alt1());
        let z = buf.g2_alt2();
        let ctrl = buf.g1() != 0;
        let x = buf.g2();
        OpObj3 { obj, x, z, ctrl }
    }
}

#[cfg(since_647)]
#[cfg(before_742)]
impl ClientProtEncode for OpObj3 {
    const PROT: ClientProt = ClientProt::OpObj3;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.obj.g2());
        buf.p2_alt2(self.z);
        buf.p1(self.ctrl as u8);
        buf.p2(self.x);
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpObj3 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let ctrl = buf.g1() != 0;
        let x = buf.g2_alt1();
        let obj = TypeId::from_g2(buf.g2_alt2());
        let z = buf.g2_alt3();
        OpObj3 { obj, x, z, ctrl }
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpObj3 {
    const PROT: ClientProt = ClientProt::OpObj3;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.ctrl as u8);
        buf.p2_alt1(self.x);
        buf.p2_alt2(self.obj.g2());
        buf.p2_alt3(self.z);
    }
}
