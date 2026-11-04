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
#[server_prot(VarcLarge, Immediate, Fixed)]
pub struct VarcLarge {
    pub id: u16,
    pub val: i32,
}

#[cfg(since_647)]
impl ServerProtMessage for VarcLarge {
    fn encode(&self, buf: &mut Packet) {
        buf.ip4(self.val);
        buf.ip2_alt1(self.id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.id) + size_of_val(&self.val)
    }
}
