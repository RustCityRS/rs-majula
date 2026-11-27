#[cfg(since_530)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_530)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_530)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_530)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_530)]
use rs_protocol_macros::server_prot;
#[cfg(since_530)]
use rs_ver::STRING_TERMINATOR;
#[cfg(since_647)]
use rs_ver::TypeId;

#[cfg(since_530)]
#[server_prot(SetMoveaction, Buffered, VarByte)]
pub struct SetMoveaction {
    pub label: String,
    #[cfg(since_647)]
    pub cursor_id: TypeId,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for SetMoveaction {
    fn encode(&self, buf: &mut Packet) {
        if !self.label.is_empty() {
            buf.pjstr(&self.label, STRING_TERMINATOR);
        }
    }

    fn sizeof(&self) -> usize {
        if self.label.is_empty() {
            0
        } else {
            Packet::sizejstr(&self.label)
        }
    }
}

#[cfg(since_647)]
impl ServerProtMessage for SetMoveaction {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr2(&self.label, STRING_TERMINATOR);
        buf.p2(self.cursor_id.g2());
    }

    fn sizeof(&self) -> usize {
        Packet::sizejstr2(&self.label) + size_of::<u16>()
    }
}
