#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;
#[cfg(since_647)]
use rs_ver::STRING_TERMINATOR;

/// Rev-647 CLIENT_SETVARCSTR_LARGE (119, var-short): a string client variable too long for
/// CLIENT_SETVARCSTR_SMALL's one-byte frame. The client reads its `g2_alt1` (the id: high byte,
/// then low byte + 128, which is rs-io's `p2_alt2`), then `gjstr` (MapAreaConfig's handler).
#[cfg(since_647)]
#[server_prot(VarcStrLarge, Buffered, VarShort)]
pub struct VarcStrLarge<'a> {
    pub id: u16,
    pub val: &'a str,
}

#[cfg(since_647)]
impl ServerProtMessage for VarcStrLarge<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt2(self.id);
        buf.pjstr(self.val, STRING_TERMINATOR);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + Packet::sizejstr(self.val)
    }
}
