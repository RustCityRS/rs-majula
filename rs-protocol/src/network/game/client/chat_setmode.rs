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

#[cfg(before_647)]
#[client_prot(Fixed(3), UserEvent)]
#[derive(Debug)]
pub struct ChatSetMode {
    pub public: u8,
    pub private: u8,
    pub trade: u8,
}

#[cfg(before_647)]
impl ClientProtMessage for ChatSetMode {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ChatSetMode {
            public: buf.g1(),
            private: buf.g1(),
            trade: buf.g1(),
        }
    }
}

#[cfg(all(since_530, before_647))]
impl ClientProtEncode for ChatSetMode {
    const PROT: ClientProt = ClientProt::ChatSetMode;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.public);
        buf.p1(self.private);
        buf.p1(self.trade);
    }
}

#[cfg(since_647)]
#[client_prot(Fixed(1), UserEvent)]
#[derive(Debug)]
pub struct ChatSetMode {
    pub mode: u8,
}

#[cfg(since_647)]
impl ClientProtMessage for ChatSetMode {
    fn decode(buf: &mut Packet, _: usize) -> Self {
        ChatSetMode { mode: buf.g1() }
    }
}

#[cfg(since_647)]
impl ClientProtEncode for ChatSetMode {
    const PROT: ClientProt = ClientProt::ChatSetMode;

    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.mode);
    }
}
