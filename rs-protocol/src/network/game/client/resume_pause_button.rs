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

#[cfg(before_530)]
#[client_prot(Fixed(2), UserEvent)]
#[derive(Debug)]
pub struct ResumePauseButton {
    pub com: u16,
}

#[cfg(before_530)]
impl ClientProtMessage for ResumePauseButton {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ResumePauseButton { com: buf.g2() }
    }
}

#[cfg(since_530)]
#[client_prot(Fixed(6), UserEvent)]
#[derive(Debug)]
pub struct ResumePauseButton {
    pub com: i32,
    pub slot: u16,
}

#[cfg(all(since_530, before_647))]
impl ClientProtMessage for ResumePauseButton {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let com = buf.g4_alt2();
        let slot = buf.g2_alt1();
        ResumePauseButton { com, slot }
    }
}

#[cfg(since_647)]
impl ClientProtMessage for ResumePauseButton {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        #[cfg(before_742)]
        let slot = buf.g2_alt1();
        let com = buf.g4_alt2();
        #[cfg(since_742)]
        let slot = buf.g2_alt3();
        ResumePauseButton { com, slot }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for ResumePauseButton {
    const PROT: ClientProt = ClientProt::ResumePauseButton;

    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.com);
        buf.p2_alt1(self.slot);
    }
}

#[cfg(since_647)]
impl ClientProtEncode for ResumePauseButton {
    const PROT: ClientProt = ClientProt::ResumePauseButton;

    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        buf.p2_alt1(self.slot);
        buf.p4_alt2(self.com);
        #[cfg(since_742)]
        buf.p2_alt3(self.slot);
    }
}
