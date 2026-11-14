#[cfg(since_530)]
use crate::markup::markup;
use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::STRING_TERMINATOR;

#[cfg(before_530)]
#[server_prot(IfSetText, Buffered, VarShort)]
pub struct IfSetText<'a> {
    pub com: u16,
    pub text: &'a str,
}

#[cfg(before_530)]
impl ServerProtMessage for IfSetText<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.pjstr(self.text, STRING_TERMINATOR);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + Packet::sizejstr(self.text)
    }
}

#[cfg(since_530)]
#[server_prot(IfSetText, Buffered, VarShort)]
pub struct IfSetText<'a> {
    pub com: i32,
    pub text: &'a str,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for IfSetText<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt3(self.com);
        buf.pjstr(&markup(self.text), STRING_TERMINATOR);
        buf.p2_alt2(self.verify);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + Packet::sizejstr(&markup(self.text)) + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetText<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.com);
        buf.pjstr(&markup(self.text), STRING_TERMINATOR);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + Packet::sizejstr(&markup(self.text))
    }
}
