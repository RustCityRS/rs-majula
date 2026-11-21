use crate::network::game::server::update_ignorelist_full::IgnoreEntry;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

pub const IGNORE_RENAMED: u8 = 0x1;
pub const IGNORE_TEMPORARY: u8 = 0x2;

#[server_prot(UpdateIgnorelistPartial, Buffered, VarByte)]
pub struct UpdateIgnorelistPartial {
    pub flags: u8,
    pub entry: IgnoreEntry,
}

impl ServerProtMessage for UpdateIgnorelistPartial {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.flags);
        self.entry.encode(buf);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.flags) + self.entry.sizeof()
    }
}
