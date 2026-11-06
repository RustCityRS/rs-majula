use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(ChatFilterSettings, Immediate, Fixed)]
pub struct ChatFilterSettings {
    pub public: u8,
    pub private: u8,
    pub trade: u8,
}

#[cfg(before_647)]
impl ServerProtMessage for ChatFilterSettings {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.public);
        buf.p1(self.private);
        buf.p1(self.trade);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.public) + size_of_val(&self.private) + size_of_val(&self.trade)
    }
}

#[cfg(since_647)]
#[server_prot(ChatFilterSettings, Buffered, Fixed)]
pub struct ChatFilterSettings {
    pub public: u8,
    pub trade: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for ChatFilterSettings {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt3(self.public);
        buf.p1_alt2(self.trade);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.public) + size_of_val(&self.trade)
    }
}
