#[cfg(rev = "225")]
use crate::network::game::server::ServerProtMessage;
#[cfg(rev = "225")]
use crate::network::game::server_prot::ServerProt;
#[cfg(rev = "225")]
use crate::network::game::server_prot_message::ServerProtMessageInfo;
#[cfg(rev = "225")]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(rev = "225")]
use rs_io::{Packet, PacketFrame};
#[cfg(rev = "225")]
use rs_protocol_macros::server_prot;

#[cfg(rev = "225")]
#[server_prot(DataLocDone, Immediate, Fixed)]
pub struct DataLocDone {
    pub x: u8,
    pub z: u8,
}

#[cfg(rev = "225")]
impl ServerProtMessage for DataLocDone {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.x);
        buf.p1(self.z);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z)
    }
}
