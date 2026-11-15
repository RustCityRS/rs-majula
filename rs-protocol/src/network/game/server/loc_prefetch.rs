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
use rs_ver::TypeId;

#[cfg(since_647)]
#[server_prot(LocPrefetch, Immediate, Fixed)]
#[derive(Debug, Clone)]
pub struct LocPrefetch {
    pub id: TypeId,
    pub shape: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for LocPrefetch {
    fn encode(&self, buf: &mut Packet) {
        #[cfg(before_742)]
        buf.p2(self.id.g2());
        #[cfg(since_742)]
        buf.p4(self.id.int());
        buf.p1(self.shape);
    }

    fn sizeof(&self) -> usize {
        #[cfg(before_742)]
        let id = size_of::<u16>();
        #[cfg(since_742)]
        let id = size_of::<i32>();
        id + size_of_val(&self.shape)
    }
}
