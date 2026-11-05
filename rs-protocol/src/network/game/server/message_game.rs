#[cfg(since_647)]
use crate::markup::markup;
use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessageInfo, jstr_len};
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
use rs_ver::STRING_TERMINATOR;

#[cfg(before_647)]
#[server_prot(MessageGame, Immediate, VarByte)]
pub struct MessageGame<'a> {
    pub text: &'a str,
}

#[cfg(before_647)]
impl ServerProtMessage for MessageGame<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(self.text, STRING_TERMINATOR);
    }

    fn sizeof(&self) -> usize {
        jstr_len(self.text)
    }
}

#[cfg(since_647)]
#[server_prot(MessageGame, Immediate, VarByte)]
pub struct MessageGame<'a> {
    pub kind: i32,
    pub text: &'a str,
}

#[cfg(since_647)]
impl MessageGame<'_> {
    /// The message as 2011's client takes it: type, the name it is from (a
    /// request's) and its text. 289's client read a trade request off the
    /// end of a game message, `<name>:tradereq:`; 2011's is message type 100
    /// from that name, which its chat box offers to accept (clientscript 84,
    /// "Accept trade"; void-634 `ChatType.TradeRequest`).
    fn parts(&self) -> (i32, Option<&str>, std::borrow::Cow<'_, str>) {
        if self.kind == 0
            && let Some(name) = self.text.strip_suffix(":tradereq:")
        {
            return (100, Some(name), "wishes to trade with you.".into());
        }
        (self.kind, None, markup(self.text))
    }
}

#[cfg(since_647)]
impl ServerProtMessage for MessageGame<'_> {
    fn encode(&self, buf: &mut Packet) {
        let (msg_type, name, text) = self.parts();
        if msg_type < 128 {
            buf.p1(msg_type as u8);
        } else {
            buf.p2((msg_type as u16) | 0x8000);
        }
        buf.p4(0); // hasPrefix
        buf.p1(name.is_some() as u8);
        if let Some(name) = name {
            buf.pjstr(name, STRING_TERMINATOR);
        }
        buf.pjstr(&text, STRING_TERMINATOR);
    }

    fn sizeof(&self) -> usize {
        let (msg_type, name, text) = self.parts();
        let smart = if msg_type < 128 { 1 } else { 2 };
        smart + 4 + 1 + name.map_or(0, jstr_len) + jstr_len(&text)
    }
}
