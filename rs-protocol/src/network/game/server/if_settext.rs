use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(before_647)]
use crate::network::game::server_prot_message::jstr_len;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetText, Buffered, VarShort)]
pub struct IfSetText<'a> {
    pub com: u16,
    pub text: &'a str,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetText<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.pjstr(self.text, 10);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + jstr_len(self.text)
    }
}

// `IfSetText` — set text of an interface component. Rev-647 opcode 34.
// Layout: `[p4 component_hash][pstr text]` — VarShort framing.
//
// VarShort framing — payload length varies with `text`.

#[cfg(since_647)]
#[server_prot(IfSetText, Buffered, VarShort)]
pub struct IfSetText {
    pub component_hash: i32,
    pub text: String,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetText {
    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.component_hash);
        buf.pjstr(&crate::markup::markup(&self.text), 0);
    }

    fn sizeof(&self) -> usize {
        5 + crate::markup::markup(&self.text).len()
    }
}
