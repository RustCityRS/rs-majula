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
use rs_ver::STRING_TERMINATOR;

#[cfg(since_530)]
#[server_prot(VarcStrSmall, Buffered, VarByte)]
pub struct VarcStrSmall<'a> {
    pub id: u16,
    pub val: &'a str,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(since_530)]
impl VarcStrSmall<'_> {
    /// The frame size this packet would need for `val`: above 255 it does not fit, and the
    /// string goes as CLIENT_SETVARCSTR_LARGE instead.
    pub fn sizeof_str(val: &str) -> usize {
        #[cfg(all(since_530, before_647))]
        let header = size_of::<u16>() << 1;
        #[cfg(since_647)]
        let header = size_of::<u16>();
        Packet::sizejstr(val) + header
    }

    /// Whether `val` fits this packet's one-byte frame size.
    pub fn fits(val: &str) -> bool {
        Self::sizeof_str(val) <= u8::MAX as usize
    }
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for VarcStrSmall<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.id);
        buf.p2_alt2(self.verify);
        buf.pjstr(self.val, STRING_TERMINATOR);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.verify) + Packet::sizejstr(self.val)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for VarcStrSmall<'_> {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(since_742)]
        buf.p2(self.id);
        buf.pjstr(self.val, STRING_TERMINATOR);
        #[cfg(before_742)]
        buf.p2_alt3(self.id);
    }

    fn sizeof(&self) -> usize {
        Packet::sizejstr(self.val) + size_of_val(&self.id)
    }
}
