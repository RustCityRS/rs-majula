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

#[client_prot(Fixed(6), ClientEvent)]
#[derive(Debug)]
pub struct WindowStatus {
    pub mode: u8,
    pub width: u16,
    pub height: u16,
    pub antialias: u8,
}

impl ClientProtMessage for WindowStatus {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let mode = buf.g1();
        let width = buf.g2();
        let height = buf.g2();
        let antialias = buf.g1();
        WindowStatus {
            mode,
            width,
            height,
            antialias,
        }
    }
}

#[cfg(since_530)]
impl ClientProtEncode for WindowStatus {
    const PROT: ClientProt = ClientProt::WindowStatus;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.mode);
        buf.p2(self.width);
        buf.p2(self.height);
        buf.p1(self.antialias);
    }
}
