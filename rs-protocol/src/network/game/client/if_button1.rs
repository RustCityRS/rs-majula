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

#[cfg_attr(before_647, client_prot(Fixed(6), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(8), UserEvent))]
#[derive(Debug)]
pub struct IfButton1 {
    pub com: i32,
    pub slot: u16,
    #[cfg(since_647)]
    pub obj: TypeId,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for IfButton1 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4s();
        let slot = buf.g2();
        IfButton1 { com, slot }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for IfButton1 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let (com, obj, slot) = (buf.g4s(), buf.g2_alt3(), buf.g2_alt1());
        #[cfg(since_742)]
        let (obj, slot, com) = (buf.g2(), buf.g2_alt3(), buf.g4_alt2());
        IfButton1 {
            com,
            slot,
            obj: TypeId::from_g2(obj),
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for IfButton1 {
    const PROT: ClientProt = ClientProt::IfButton1;

    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.com);
        buf.p2(self.slot);
    }
}

#[cfg(since_647)]
impl ClientProtEncode for IfButton1 {
    const PROT: ClientProt = ClientProt::IfButton1;

    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p4(self.com);
            buf.p2_alt3(self.obj.g2());
            buf.p2_alt1(self.slot);
        }
        #[cfg(since_742)]
        {
            buf.p2(self.obj.g2());
            buf.p2_alt3(self.slot);
            buf.p4_alt2(self.com);
        }
    }
}
