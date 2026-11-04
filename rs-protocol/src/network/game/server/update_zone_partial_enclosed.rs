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

#[cfg(since_647)]
#[server_prot(UpdateZonePartialEnclosed, Immediate, VarShort)]
pub struct UpdateZonePartialEnclosed<'a> {
    pub x: u8,
    pub z: u8,
    pub y: u8,
    pub bytes: &'a [u8],
}

#[cfg(since_647)]
impl ServerProtMessage for UpdateZonePartialEnclosed<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.x);
        buf.p1_alt1(self.y);
        buf.p1(self.z);
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z) + size_of_val(&self.y) + self.bytes.len()
    }
}
