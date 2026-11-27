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

#[cfg_attr(before_647, client_prot(Fixed(10), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(12), UserEvent))]
#[derive(Debug)]
pub struct ApCoordT {
    pub x: u16,
    pub z: u16,
    #[cfg(since_647)]
    pub obj: TypeId,
    pub slot: u16,
    pub com: i32,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for ApCoordT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4_alt3();
        let x = buf.g2_alt2();
        let slot = buf.g2_alt3();
        let z = buf.g2_alt2();
        ApCoordT { x, z, slot, com }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for ApCoordT {
    const PROT: ClientProt = ClientProt::ApCoordT;

    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt3(self.com);
        buf.p2_alt2(self.x);
        buf.p2_alt3(self.slot);
        buf.p2_alt2(self.z);
    }
}

#[cfg(since_647)]
impl ClientProtMessage for ApCoordT {
    #[cfg(before_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let x = buf.g2_alt1();
        let slot = buf.g2();
        let obj = TypeId::from_g2(buf.g2_alt2());
        let z = buf.g2_alt2();
        let com = buf.g4s();
        ApCoordT {
            x,
            z,
            obj,
            slot,
            com,
        }
    }

    #[cfg(since_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = TypeId::from_g2(buf.g2());
        let slot = buf.g2();
        let x = buf.g2_alt3();
        let com = buf.g4_alt3();
        let z = buf.g2_alt3();
        ApCoordT {
            x,
            z,
            obj,
            slot,
            com,
        }
    }
}

#[cfg(since_647)]
impl ClientProtEncode for ApCoordT {
    const PROT: ClientProt = ClientProt::ApCoordT;

    #[cfg(before_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.x);
        buf.p2(self.slot);
        buf.p2_alt2(self.obj.g2());
        buf.p2_alt2(self.z);
        buf.p4(self.com);
    }

    #[cfg(since_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.obj.g2());
        buf.p2(self.slot);
        buf.p2_alt3(self.x);
        buf.p4_alt3(self.com);
        buf.p2_alt3(self.z);
    }
}
