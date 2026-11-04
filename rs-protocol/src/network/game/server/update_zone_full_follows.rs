use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(UpdateZoneFullFollows, Immediate, Fixed)]
pub struct UpdateZoneFullFollows {
    pub x: u8,
    pub z: u8,
}

#[cfg(before_647)]
impl ServerProtMessage for UpdateZoneFullFollows {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.x);
        buf.p1(self.z);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z)
    }
}

// `UpdateZoneFullFollows` — announces the zone header for subsequent
// zone events sent to the client in the current tick.
// Rev-647 opcode 75 (Fixed 3). Layout: `[p1_alt3 level][p1 x_off][p1_alt1 z_off]`.

#[cfg(since_647)]
#[server_prot(UpdateZoneFullFollows, Buffered, Fixed)]
pub struct UpdateZoneFullFollows {
    pub x_off: u8,
    pub z_off: u8,
    pub level: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateZoneFullFollows {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt3(self.level);
        buf.p1(self.x_off);
        buf.p1_alt1(self.z_off);
    }

    fn sizeof(&self) -> usize {
        use std::mem::size_of_val;
        size_of_val(&self.x_off) + size_of_val(&self.z_off) + size_of_val(&self.level)
    }
}
