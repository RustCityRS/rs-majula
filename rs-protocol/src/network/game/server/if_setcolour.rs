use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(IfSetColour, Buffered, Fixed)]
pub struct IfSetColour {
    pub com: u16,
    pub colour: u16,
}

#[cfg(before_647)]
impl ServerProtMessage for IfSetColour {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.colour);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.colour)
    }
}

#[cfg(since_647)]
#[server_prot(IfSetColour, Buffered, Fixed)]
pub struct IfSetColour {
    pub com: i32,
    pub colour: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetColour {
    // Client reads `g4_alt2 component_hash` then `ig2_alt1 colour`
    // (`MapAreaConfig.java`, IF_SETCOLOUR). `ip4`/`p2` wrote the right number
    // of bytes in the wrong order, so the component hash and colour both
    // arrived scrambled.
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.com);
        buf.ip2_alt1(self.colour);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.colour)
    }
}
