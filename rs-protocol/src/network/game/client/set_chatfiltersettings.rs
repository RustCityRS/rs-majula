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
pub struct SetChatfiltersettings {
    pub public: u8,
    pub private: u8,
    pub trade: u8,
}

impl ClientProtMessage for SetChatfiltersettings {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        let public = buf.g1();
        let private = buf.g1();
        let trade = buf.g1();
        SetChatfiltersettings {
            public,
            private,
            trade,
        }
    }
}

#[cfg(since_530)]
impl ClientProtEncode for SetChatfiltersettings {
    const PROT: ClientProt = ClientProt::SetChatfiltersettings;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.public);
        buf.p1(self.private);
        buf.p1(self.trade);
    }
}
