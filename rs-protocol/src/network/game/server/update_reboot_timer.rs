use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(UpdateRebootTimer, Buffered, Fixed)] // TODO: what should priority be?
pub struct UpdateRebootTimer {
    pub clocks: u16,
}

impl ServerProtMessage for UpdateRebootTimer {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.clocks);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.clocks)
    }
}
