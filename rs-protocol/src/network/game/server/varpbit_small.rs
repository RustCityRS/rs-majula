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
#[server_prot(VarpBitSmall, Immediate, Fixed)]
pub struct VarpBitSmall {
    pub id: u16,
    pub val: u8,
}

#[cfg(since_530)]
impl ServerProtMessage for VarpBitSmall {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        {
            buf.p1_alt1(self.val);
            buf.p2_alt1(self.id);
        }
        #[cfg(since_742)]
        {
            buf.p2_alt3(self.id);
            buf.p1_alt2(self.val);
        }
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}
