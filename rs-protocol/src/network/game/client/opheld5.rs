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
pub struct OpHeld5 {
    pub obj: TypeId,
    pub slot: u16,
    #[cfg(before_530)]
    pub com: u16,
    #[cfg(since_530)]
    pub com: i32,
}

#[cfg(before_530)]
impl ClientProtMessage for OpHeld5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpHeld5 {
            obj: TypeId::from_g2(buf.g2()),
            slot: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_530)]
impl ClientProtMessage for OpHeld5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = TypeId::from_g2(buf.g2_alt2());
        let slot = buf.g2_alt2();
        let com = buf.g4_alt3();
        OpHeld5 { obj, slot, com }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpHeld5 {
    const PROT: ClientProt = ClientProt::OpHeld5;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt2(self.obj.g2());
        buf.p2_alt2(self.slot);
        buf.p4_alt3(self.com);
    }
}
