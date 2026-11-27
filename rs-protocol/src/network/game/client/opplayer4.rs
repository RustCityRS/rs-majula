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
pub struct OpPlayer4 {
    pub pid: u16,
    #[cfg(since_647)]
    pub ctrl: bool,
}

#[cfg(before_530)]
impl ClientProtMessage for OpPlayer4 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayer4 { pid: buf.g2() }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for OpPlayer4 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        OpPlayer4 { pid: buf.g2_alt3() }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for OpPlayer4 {
    const PROT: ClientProt = ClientProt::OpPlayer4;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt3(self.pid);
    }
}

#[cfg(since_647)]
#[cfg(before_742)]
impl ClientProtMessage for OpPlayer4 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let pid = buf.g2_alt2();
        let ctrl = buf.g1() != 0;
        OpPlayer4 { pid, ctrl }
    }
}

#[cfg(since_647)]
#[cfg(before_742)]
impl ClientProtEncode for OpPlayer4 {
    const PROT: ClientProt = ClientProt::OpPlayer4;

    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt2(self.pid);
        buf.p1(self.ctrl as u8);
    }
}

#[cfg(since_742)]
impl ClientProtMessage for OpPlayer4 {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let (ctrl, pid) = (buf.g1() != 0, buf.g2_alt1());
        OpPlayer4 { pid, ctrl }
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpPlayer4 {
    const PROT: ClientProt = ClientProt::OpPlayer4;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.ctrl as u8);
        buf.p2_alt1(self.pid);
    }
}
