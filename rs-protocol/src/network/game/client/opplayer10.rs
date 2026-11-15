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

#[client_prot(Fixed(3), UserEvent)]
#[derive(Debug)]
pub struct OpPlayer10 {
    pub pid: u16,
    pub ctrl: bool,
}

impl ClientProtMessage for OpPlayer10 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let (ctrl, pid) = (buf.g1_alt2() != 0, buf.g2());
        #[cfg(since_742)]
        let (ctrl, pid) = (buf.g1() != 0, buf.g2_alt1());
        OpPlayer10 { pid, ctrl }
    }
}

#[cfg(all(since_530, before_742))]
impl ClientProtEncode for OpPlayer10 {
    const PROT: ClientProt = ClientProt::OpPlayer10;

    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.ctrl as u8);
        buf.p2(self.pid);
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpPlayer10 {
    const PROT: ClientProt = ClientProt::OpPlayer10;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.ctrl as u8);
        buf.p2_alt1(self.pid);
    }
}
