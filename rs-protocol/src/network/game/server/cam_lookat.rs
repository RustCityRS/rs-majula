#[cfg(since_647)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_647)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_647)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_647)]
use rs_protocol_macros::server_prot;

#[cfg(since_647)]
#[server_prot(CamLookat, Buffered, Fixed)]
pub struct CamLookat {
    pub local_x: u8,
    pub local_z: u8,
    pub height: u16,
    pub const_speed: u8,
    pub var_speed: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for CamLookat {
    fn encode(&self, buf: &mut Packet) {
        // `MapAreaConfig`'s CAM_LOOKAT branch, into `AClass13.method622`.
        // Height is in 1/128-tile units, as before 647; the client scales it
        // (`<< 2`) to its 512-unit tiles.
        buf.p1_alt1(self.local_x);
        buf.ip2_alt1(self.height);
        buf.p1_alt2(self.local_z);
        buf.p1_alt1(self.const_speed);
        buf.p1_alt2(self.var_speed);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.local_x)
            + size_of_val(&self.local_z)
            + size_of_val(&self.height)
            + size_of_val(&self.const_speed)
            + size_of_val(&self.var_speed)
    }
}
