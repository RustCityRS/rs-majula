#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::jstr_len;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;

#[cfg(since_647)]
#[server_prot(VarcStrSmall, Buffered, VarByte)]
pub struct VarcStrSmall<'a> {
    pub id: u16,
    pub val: &'a str,
}

#[cfg(since_647)]
impl ServerProtMessage for VarcStrSmall<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.pjstr(&self.val, 0);
        buf.ip2_alt1(self.id);
    }

    fn sizeof(&self) -> usize {
        jstr_len(&self.val) + size_of_val(&self.id)
    }
}
