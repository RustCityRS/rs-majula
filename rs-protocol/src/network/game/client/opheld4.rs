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

#[cfg_attr(before_530, client_prot(Fixed(6), UserEvent))]
#[cfg_attr(since_530, client_prot(Fixed(8), UserEvent))]
#[derive(Debug)]
pub struct OpHeld4 {
    pub obj: TypeId,
    pub slot: u16,
    #[cfg(before_530)]
    pub com: u16,
    #[cfg(since_530)]
    pub com: i32,
}

#[cfg(before_530)]
impl ClientProtMessage for OpHeld4 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpHeld4 {
            obj: TypeId::from_g2(buf.g2()),
            slot: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_530)]
impl ClientProtMessage for OpHeld4 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4_alt1();
        let obj = TypeId::from_g2(buf.g2_alt3());
        let slot = buf.g2_alt3();
        OpHeld4 { obj, slot, com }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpHeld4 {
    const PROT: ClientProt = ClientProt::OpHeld4;

    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt1(self.com);
        buf.p2_alt3(self.obj.g2());
        buf.p2_alt3(self.slot);
    }
}
