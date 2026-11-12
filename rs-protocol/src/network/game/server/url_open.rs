use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

/// A page for the client to open in the browser. The whole frame is the
/// URL, CP-1252 with no terminator, so `url` must be ASCII.
#[server_prot(UrlOpen, Immediate, VarByte)]
pub struct UrlOpen<'a> {
    pub url: &'a str,
}

impl ServerProtMessage for UrlOpen<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.pdata(self.url.as_bytes(), 0, self.url.len());
    }

    fn sizeof(&self) -> usize {
        self.url.len()
    }
}
