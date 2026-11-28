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

#[cfg_attr(before_647, client_prot(Fixed(12), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(16), UserEvent))]
#[derive(Debug)]
pub struct IfButtonD {
    pub com: i32,
    pub slot: u16,
    #[cfg(since_647)]
    pub obj: TypeId,
    pub com2: i32,
    pub slot2: u16,
    #[cfg(since_647)]
    pub obj2: TypeId,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for IfButtonD {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4_alt3();
        let slot2 = buf.g2_alt1();
        let com2 = buf.g4s();
        let slot = buf.g2_alt1();
        IfButtonD {
            com,
            slot,
            com2,
            slot2,
        }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for IfButtonD {
    #[cfg(before_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4s();
        let slot = buf.g2_alt3();
        let slot2 = buf.g2_alt3();
        let com2 = buf.g4_alt2();
        let obj = TypeId::from_g2(buf.g2_alt1());
        let obj2 = TypeId::from_g2(buf.g2());
        IfButtonD {
            com,
            slot,
            obj,
            com2,
            slot2,
            obj2,
        }
    }

    #[cfg(since_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com2 = buf.g4_alt1();
        let obj = TypeId::from_g2(buf.g2_alt2());
        let com = buf.g4s();
        let slot2 = buf.g2_alt1();
        let slot = buf.g2_alt2();
        let obj2 = TypeId::from_g2(buf.g2_alt3());
        IfButtonD {
            com,
            slot,
            obj,
            com2,
            slot2,
            obj2,
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for IfButtonD {
    const PROT: ClientProt = ClientProt::IfButtonD;

    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt3(self.com);
        buf.p2_alt1(self.slot2);
        buf.p4(self.com2);
        buf.p2_alt1(self.slot);
    }
}

#[cfg(since_647)]
impl ClientProtEncode for IfButtonD {
    const PROT: ClientProt = ClientProt::IfButtonD;

    #[cfg(before_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.com);
        buf.p2_alt3(self.slot);
        buf.p2_alt3(self.slot2);
        buf.p4_alt2(self.com2);
        buf.p2_alt1(self.obj.g2());
        buf.p2(self.obj2.g2());
    }

    #[cfg(since_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt1(self.com2);
        buf.p2_alt2(self.obj.g2());
        buf.p4(self.com);
        buf.p2_alt1(self.slot2);
        buf.p2_alt2(self.slot);
        buf.p2_alt3(self.obj2.g2());
    }
}
