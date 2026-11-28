use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_530)]
#[server_prot(IfSetColour, Buffered, Fixed)]
pub struct IfSetColour {
    pub com: u16,
    pub colour: u16,
}

#[cfg(before_530)]
impl ServerProtMessage for IfSetColour {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
        buf.p2(self.colour);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.colour)
    }
}

#[cfg(since_530)]
#[server_prot(IfSetColour, Buffered, Fixed)]
pub struct IfSetColour {
    pub com: i32,
    pub colour: u16,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for IfSetColour {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.com);
        buf.p2_alt2(self.verify);
        buf.p2_alt3(self.colour);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.colour) + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for IfSetColour {
    // Client reads `g4_alt2 component_hash` then `g2_alt3 colour`
    // (`MapAreaConfig.java`, IF_SETCOLOUR). `p4_alt1`/`p2` wrote the right number
    // of bytes in the wrong order, so the component hash and colour both
    // arrived scrambled.
    #[cfg(before_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt2(self.com);
        buf.p2_alt3(self.colour);
    }

    #[cfg(since_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt2(self.colour);
        buf.p4_alt2(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com) + size_of_val(&self.colour)
    }
}
