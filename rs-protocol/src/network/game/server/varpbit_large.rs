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
#[server_prot(VarpBitLarge, Immediate, Fixed)]
pub struct VarpBitLarge {
    pub id: u16,
    pub val: i32,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for VarpBitLarge {
    fn encode(&self, buf: &mut Packet) {
        buf.p4_alt1(self.val);
        buf.p2_alt3(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for VarpBitLarge {
    #[cfg(before_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.id);
        buf.p4_alt1(self.val);
    }

    #[cfg(since_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p4(self.val);
        buf.p2_alt3(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}
