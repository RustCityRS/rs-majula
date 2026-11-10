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
use rs_ver::STRING_TERMINATOR;

#[cfg(since_647)]
#[server_prot(VarcStrSmall, Buffered, VarByte)]
pub struct VarcStrSmall<'a> {
    pub id: u16,
    pub val: &'a str,
}

#[cfg(since_647)]
impl VarcStrSmall<'_> {
    /// The frame size this packet would need for `val`: above 255 it does not fit, and the
    /// string goes as CLIENT_SETVARCSTR_LARGE instead.
    pub fn sizeof_str(val: &str) -> usize {
        Packet::sizejstr(val) + size_of::<u16>()
    }
}

#[cfg(since_647)]
impl ServerProtMessage for VarcStrSmall<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(self.val, STRING_TERMINATOR);
        buf.p2_alt3(self.id);
    }

    fn sizeof(&self) -> usize {
        Packet::sizejstr(self.val) + size_of_val(&self.id)
    }
}
