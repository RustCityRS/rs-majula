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

#[cfg(before_530)]
#[client_prot(Fixed(4), UserEvent)]
#[derive(Debug)]
pub struct OpPlayerT {
    pub pid: u16,
    pub com: u16,
}

#[cfg(before_530)]
impl ClientProtMessage for OpPlayerT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayerT {
            pid: buf.g2(),
            com: buf.g2(),
        }
    }
}

#[cfg(since_530)]
#[cfg_attr(before_647, client_prot(Fixed(8), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(11), UserEvent))]
#[derive(Debug)]
pub struct OpPlayerT {
    pub pid: u16,
    #[cfg(since_647)]
    pub obj: TypeId,
    pub slot: u16,
    pub com: i32,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpPlayerT {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let slot = buf.g2_alt2();
        let com = buf.g4_alt1();
        let pid = buf.g2_alt3();
        OpPlayerT { pid, slot, com }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpPlayerT {
    const PROT: ClientProt = ClientProt::OpPlayerT;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt2(self.slot);
        buf.p4_alt1(self.com);
        buf.p2_alt3(self.pid);
    }
}

#[cfg(since_647)]
impl ClientProtMessage for OpPlayerT {
    #[cfg(before_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let pid = buf.g2_alt2();
        let slot = buf.g2_alt1();
        let com = buf.g4s();
        let ctrl = buf.g1_alt3() != 0;
        let obj = TypeId::from_g2(buf.g2_alt1());
        OpPlayerT {
            pid,
            obj,
            slot,
            com,
            ctrl,
        }
    }

    #[cfg(since_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let obj = TypeId::from_g2(buf.g2_alt1());
        let ctrl = buf.g1_alt3() != 0;
        let pid = buf.g2_alt3();
        let com = buf.g4_alt2();
        let slot = buf.g2_alt1();
        OpPlayerT {
            pid,
            obj,
            slot,
            com,
            ctrl,
        }
    }
}

#[cfg(since_647)]
impl ClientProtEncode for OpPlayerT {
    const PROT: ClientProt = ClientProt::OpPlayerT;

    #[cfg(before_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt2(self.pid);
        buf.p2_alt1(self.slot);
        buf.p4(self.com);
        buf.p1_alt3(self.ctrl as u8);
        buf.p2_alt1(self.obj.g2());
    }

    #[cfg(since_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.obj.g2());
        buf.p1_alt3(self.ctrl as u8);
        buf.p2_alt3(self.pid);
        buf.p4_alt2(self.com);
        buf.p2_alt1(self.slot);
    }
}
