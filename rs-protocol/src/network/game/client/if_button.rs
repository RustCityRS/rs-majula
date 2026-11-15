#[cfg(all(since_530, before_647))]
use crate::network::game::client::ClientProtEncode;
use crate::network::game::client::ClientProtMessage;
#[cfg(all(since_530, before_647))]
use crate::network::game::client_prot::ClientProt;
use crate::network::game::client_prot_category::ClientProtCategory;
use crate::network::game::client_prot_frame::ClientProtFrame;
use crate::network::game::client_prot_message::ClientProtMessageInfo;
use rs_io::Packet;
use rs_protocol_macros::client_prot;

#[cfg_attr(before_530, client_prot(Fixed(2), UserEvent))]
#[cfg_attr(since_530, client_prot(Fixed(4), UserEvent))]
#[derive(Debug)]
pub struct IfButton {
    #[cfg(before_530)]
    pub com: u16,
    #[cfg(since_530)]
    pub com: i32,
}

#[cfg(before_530)]
impl ClientProtMessage for IfButton {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        IfButton { com: buf.g2() }
    }
}

#[cfg(since_530)]
impl ClientProtMessage for IfButton {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        IfButton { com: buf.g4s() }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for IfButton {
    const PROT: ClientProt = ClientProt::IfButton;

    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.com);
    }
}
