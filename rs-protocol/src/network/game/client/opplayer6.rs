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
pub struct OpPlayer6 {
    pub pid: u16,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpPlayer6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayer6 { pid: buf.g2_alt1() }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for OpPlayer6 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let (ctrl, pid) = (buf.g1() != 0, buf.g2_alt2());
        #[cfg(since_742)]
        let (ctrl, pid) = (buf.g1() != 0, buf.g2_alt1());
        OpPlayer6 { pid, ctrl }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpPlayer6 {
    const PROT: ClientProt = ClientProt::OpPlayer6;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.pid);
    }
}

#[cfg(all(since_647, before_742))]
impl ClientProtEncode for OpPlayer6 {
    const PROT: ClientProt = ClientProt::OpPlayer6;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.ctrl as u8);
        buf.p2_alt2(self.pid);
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpPlayer6 {
    const PROT: ClientProt = ClientProt::OpPlayer6;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.ctrl as u8);
        buf.p2_alt1(self.pid);
    }
}
