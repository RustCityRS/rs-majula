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

#[cfg_attr(before_647, client_prot(Fixed(2), UserEvent))]
#[cfg_attr(since_647, client_prot(Fixed(3), UserEvent))]
#[derive(Debug)]
pub struct OpNpc5 {
    pub nid: u16,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(before_530)]
impl ClientProtMessage for OpNpc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpNpc5 { nid: buf.g2() }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpNpc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpNpc5 { nid: buf.g2_alt1() }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpNpc5 {
    const PROT: ClientProt = ClientProt::OpNpc5;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.nid);
    }
}

#[cfg(since_647)]
impl ClientProtMessage for OpNpc5 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let (ctrl, nid) = (buf.g1_alt2() != 0, buf.g2_alt1());
        #[cfg(since_742)]
        let (ctrl, nid) = (buf.g1() != 0, buf.g2_alt1());
        OpNpc5 { nid, ctrl }
    }
}

#[cfg(since_647)]
impl ClientProtEncode for OpNpc5 {
    const PROT: ClientProt = ClientProt::OpNpc5;

    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p1_alt2(self.ctrl as u8);
            buf.p2_alt1(self.nid);
        }
        #[cfg(since_742)]
        {
            buf.p1(self.ctrl as u8);
            buf.p2_alt1(self.nid);
        }
    }
}
