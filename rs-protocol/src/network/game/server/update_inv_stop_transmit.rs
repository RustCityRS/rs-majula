use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_530)]
#[server_prot(UpdateInvStopTransmit, Immediate, Fixed)]
pub struct UpdateInvStopTransmit {
    pub com: u16,
}

#[cfg(before_530)]
impl ServerProtMessage for UpdateInvStopTransmit {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.com);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.com)
    }
}

#[cfg(all(since_530, before_647))]
#[server_prot(UpdateInvStopTransmit, Immediate, Fixed)]
pub struct UpdateInvStopTransmit {
    pub inv_id: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for UpdateInvStopTransmit {
    fn encode(&self, buf: &mut Packet) {
        buf.p2_alt1(self.inv_id);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.inv_id)
    }
}
