use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
use crate::network::game::server_prot_priority::ServerProtPriority;

#[server_prot(ClanchannelDelta, Buffered, VarShort)]
pub struct ClanchannelDelta {
    pub affined: bool,
    pub data: Vec<u8>,
}

impl ServerProtMessage for ClanchannelDelta {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(if self.affined { 1 } else { 0 });
        buf.pdata(&self.data, 0, self.data.len());
    }

    fn sizeof(&self) -> usize {
        1 + self.data.len()
    }
}
