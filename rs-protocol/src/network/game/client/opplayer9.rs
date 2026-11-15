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
pub struct OpPlayer9 {
    pub pid: u16,
    pub ctrl: bool,
}

impl ClientProtMessage for OpPlayer9 {
    #[cfg(before_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let pid = buf.g2();
        let ctrl = buf.g1_alt1() != 0;
        OpPlayer9 { pid, ctrl }
    }

    #[cfg(since_742)]
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let ctrl = buf.g1() != 0;
        let pid = buf.g2_alt1();
        OpPlayer9 { pid, ctrl }
    }
}

#[cfg(all(since_530, before_742))]
impl ClientProtEncode for OpPlayer9 {
    const PROT: ClientProt = ClientProt::OpPlayer9;

    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.pid);
        buf.p1_alt1(self.ctrl as u8);
    }
}

#[cfg(since_742)]
impl ClientProtEncode for OpPlayer9 {
    const PROT: ClientProt = ClientProt::OpPlayer9;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.ctrl as u8);
        buf.p2_alt1(self.pid);
    }
}
