use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(VarpSmall, Immediate, Fixed)]
pub struct VarpSmall {
    pub id: u16,
    pub val: u8,
}

impl VarpSmall {
    /// Whether `val` fits this packet's byte, which the client reads signed;
    /// a larger one goes as VARP_LARGE.
    pub fn fits(val: i32) -> bool {
        i8::try_from(val).is_ok()
    }
}

#[cfg(before_530)]
impl ServerProtMessage for VarpSmall {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.id);
        buf.p1(self.val);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for VarpSmall {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt2(self.id);
        buf.p1_alt2(self.val);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for VarpSmall {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p1_alt3(self.val);
            buf.p2(self.id);
        }
        #[cfg(since_742)]
        {
            buf.p1_alt1(self.val);
            buf.p2_alt1(self.id);
        }
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}
