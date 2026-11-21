use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
#[cfg(since_742)]
use rs_ver::STRING_TERMINATOR;

#[cfg(before_647)]
#[server_prot(UpdateIgnoreList, Buffered, VarShort)]
pub struct UpdateIgnoreList<'a> {
    pub users37: &'a [i64],
}

#[cfg(before_647)]
impl ServerProtMessage for UpdateIgnoreList<'_> {
    fn encode(&self, buf: &mut Packet) {
        for name in self.users37 {
            buf.p8(*name);
        }
    }

    fn sizeof(&self) -> usize {
        8 * self.users37.len()
    }
}

#[cfg(since_742)]
pub const IGNORE_RENAMED: u8 = 0x1;
#[cfg(since_742)]
pub const IGNORE_TEMPORARY: u8 = 0x2;

#[cfg(since_742)]
pub struct IgnoreEntry {
    pub flags: u8,
    pub name: String,
    pub prev_name: String,
}

#[cfg(since_742)]
#[server_prot(UpdateIgnoreList, Buffered, VarShort)]
pub struct UpdateIgnoreList {
    pub entries: Vec<IgnoreEntry>,
}

#[cfg(since_742)]
impl ServerProtMessage for UpdateIgnoreList {
    fn encode(&self, buf: &mut Packet) {
        for entry in &self.entries {
            buf.p1(entry.flags);
            buf.pjstr(&entry.name, STRING_TERMINATOR);
            buf.pjstr(&entry.prev_name, STRING_TERMINATOR);
        }
    }

    fn sizeof(&self) -> usize {
        self.entries
            .iter()
            .map(|e| {
                size_of_val(&e.flags) + Packet::sizejstr(&e.name) + Packet::sizejstr(&e.prev_name)
            })
            .sum()
    }
}
