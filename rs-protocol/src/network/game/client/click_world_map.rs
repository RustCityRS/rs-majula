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
pub struct ClickWorldMap {
    pub coord: u32,
}

impl ClientProtMessage for ClickWorldMap {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ClickWorldMap {
            #[cfg(before_742)]
            coord: buf.g4_alt2() as u32,
            #[cfg(since_742)]
            coord: buf.g4_alt1() as u32,
        }
    }
}

#[cfg(since_530)]
impl ClientProtEncode for ClickWorldMap {
    const PROT: ClientProt = ClientProt::ClickWorldMap;

    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        buf.p4_alt2(self.coord as i32);
        #[cfg(since_742)]
        buf.p4_alt1(self.coord as i32);
    }
}
