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
#[server_prot(DataLand, Immediate, VarShort)]
pub struct DataLand<'a> {
    pub x: u8,
    pub z: u8,
    pub off: u16,
    pub len: u16,
    pub data: &'a [u8],
}

#[cfg(rev = "225")]
impl ServerProtMessage for DataLand<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.x);
        buf.p1(self.z);
        buf.p2(self.off);
        buf.p2(self.len);
        buf.pdata(self.data, 0, self.data.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x)
            + size_of_val(&self.z)
            + size_of_val(&self.off)
            + size_of_val(&self.len)
            + self.data.len()
    }
}
