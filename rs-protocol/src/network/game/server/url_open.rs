use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
#[cfg(since_742)]
use rs_ver::STRING_TERMINATOR;

/// A page for the client to open in the browser, so `url` must be ASCII.
/// Before 742 the whole frame is the URL with no terminator; from 742 a flag
/// byte comes first (0: no alternative URL).
#[cfg_attr(before_742, server_prot(UrlOpen, Immediate, VarByte))]
#[cfg_attr(since_742, server_prot(UrlOpen, Immediate, VarShort))]
pub struct UrlOpen<'a> {
    pub url: &'a str,
}

impl ServerProtMessage for UrlOpen<'_> {
    #[cfg(before_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.pdata(self.url.as_bytes(), 0, self.url.len());
    }

    #[cfg(since_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p1(0);
        buf.pjstr(self.url, STRING_TERMINATOR);
    }

    #[cfg(before_742)]
    fn sizeof(&self) -> usize {
        self.url.len()
    }

    #[cfg(since_742)]
    fn sizeof(&self) -> usize {
        1 + self.url.len() + 1
    }
}
