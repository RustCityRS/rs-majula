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
#[cfg_attr(since_742, client_prot(Fixed(3), UserEvent))]
#[derive(Debug)]
pub struct OpNpc6 {
    pub npc: TypeId,
    #[cfg(since_742)]
    pub ctrl: bool,
}

#[cfg(before_742)]
impl ClientProtMessage for OpNpc6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpNpc6 {
            npc: TypeId::from_g2(buf.g2()),
        }
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpNpc6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let (ctrl, npc) = (buf.g1() != 0, buf.g2_alt1());
        OpNpc6 {
            npc: TypeId::from_g2(npc),
            ctrl,
        }
    }
}

#[cfg(all(since_530, before_742))]
impl ClientProtEncode for OpNpc6 {
    const PROT: ClientProt = ClientProt::OpNpc6;

    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.npc.g2());
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpNpc6 {
    const PROT: ClientProt = ClientProt::OpNpc6;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.ctrl as u8);
        buf.p2_alt1(self.npc.g2());
    }
}
