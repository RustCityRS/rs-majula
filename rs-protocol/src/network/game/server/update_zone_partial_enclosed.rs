use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(UpdateZonePartialEnclosed, Immediate, VarShort)]
pub struct UpdateZonePartialEnclosed<'a> {
    pub x: u8,
    pub z: u8,
    pub bytes: &'a [u8],
}

#[cfg(before_647)]
impl ServerProtMessage for UpdateZonePartialEnclosed<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.x);
        buf.p1(self.z);
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z) + self.bytes.len()
    }
}

// `UpdateZonePartialEnclosed` — rev-647 UPDATE_ZONE_PARTIAL_ENCLOSED
// (opcode 111, VarShort). Carries a pre-multiplexed batch of zone events.
// Layout: `[p1 x_off][p1_alt1 level][p1 z_off][pdata shared]`.

#[cfg(since_647)]
#[server_prot(UpdateZonePartialEnclosed, Immediate, VarShort)]
pub struct UpdateZonePartialEnclosed<'a> {
    pub x_off: u8,
    pub z_off: u8,
    pub level: u8,
    /// Pre-serialized batch of zone events (one or more `enclose`d zone
    /// messages packed together by the world's zone flush pass).
    pub bytes: &'a [u8],
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateZonePartialEnclosed<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.x_off);
        buf.p1_alt1(self.level);
        buf.p1(self.z_off);
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        3 + self.bytes.len()
    }
}
