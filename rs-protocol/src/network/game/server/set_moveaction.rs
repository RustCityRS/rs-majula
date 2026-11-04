#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::jstr_len;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;

#[cfg(since_647)]
#[server_prot(SetMoveaction, Buffered, VarByte)]
pub struct SetMoveaction {
    pub label: String,
    pub cursor_id: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for SetMoveaction {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(0);
        buf.pjstr(&self.label, 0);
        buf.p2(self.cursor_id);
    }

    fn sizeof(&self) -> usize {
        1 + jstr_len(&self.label) + size_of_val(&self.cursor_id)
    }
}
