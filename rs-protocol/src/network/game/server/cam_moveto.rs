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
#[server_prot(CamMoveto, Buffered, Fixed)]
pub struct CamMoveto {
    pub local_x: u8,
    pub local_z: u8,
    pub height: u16,
    pub const_speed: u8,
    pub var_speed: u8,
}

#[cfg(since_647)]
impl ServerProtMessage for CamMoveto {
    fn encode(&self, buf: &mut Packet) {
        // `MapAreaConfig`'s CAM_MOVETO branch, into `Class36.method269`.
        // Height in 1/128-tile units, scaled by the client as for CAM_LOOKAT.
        buf.p1_alt3(self.local_x);
        buf.p1_alt1(self.local_z);
        buf.p1_alt3(self.var_speed);
        buf.p2(self.height);
        buf.p1_alt3(self.const_speed);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.local_x)
            + size_of_val(&self.local_z)
            + size_of_val(&self.height)
            + size_of_val(&self.const_speed)
            + size_of_val(&self.var_speed)
    }
}
