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

#[client_prot(Fixed(4), UserEvent)]
#[derive(Debug)]
pub struct ResumePCountDialog {
    pub input: i32,
}

impl ClientProtMessage for ResumePCountDialog {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ResumePCountDialog { input: buf.g4s() }
    }
}

#[cfg(since_530)]
impl ClientProtEncode for ResumePCountDialog {
    const PROT: ClientProt = ClientProt::ResumePCountDialog;

    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.input);
    }
}
