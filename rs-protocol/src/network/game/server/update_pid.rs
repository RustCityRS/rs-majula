use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(rev = "225")]
#[server_prot(UpdatePid, Immediate, Fixed)] // TODO: what should priority be?
pub struct UpdatePid {
    pub pid: u16,
}

#[cfg(rev = "225")]
impl ServerProtMessage for UpdatePid {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.pid);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.pid)
    }
}

#[cfg(since_244)]
#[server_prot(UpdatePid, Immediate, Fixed)] // TODO: what should priority be?
pub struct UpdatePid {
    pub pid: u16,
    pub members: bool,
}

#[cfg(since_244)]
impl ServerProtMessage for UpdatePid {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.pid);
        buf.p1(self.members as u8);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.pid) + size_of_val(&self.members)
    }
}
