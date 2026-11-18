#[cfg(since_530)]
use crate::network::game::server_prot::ServerProt;
#[cfg(since_530)]
use crate::network::game::server_prot_message::{ServerProtMessage, ServerProtMessageInfo};
#[cfg(since_530)]
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_530)]
use rs_io::{Packet, PacketFrame};
#[cfg(since_530)]
use rs_protocol_macros::server_prot;

#[cfg(since_530)]
#[server_prot(CamMoveto, Buffered, Fixed)]
pub struct CamMoveto {
    pub local_x: u8,
    pub local_z: u8,
    pub height: u16,
    pub rate: u8,
    pub rate2: u8,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for CamMoveto {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.verify);
        buf.p1(self.local_x);
        buf.p1(self.local_z);
        buf.p2(self.height);
        buf.p1(self.rate);
        buf.p1(self.rate2);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.local_x)
            + size_of_val(&self.local_z)
            + size_of_val(&self.height)
            + size_of_val(&self.rate)
            + size_of_val(&self.rate2)
            + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for CamMoveto {
    fn encode(&self, buf: &mut Packet) {
        // `MapAreaConfig`'s CAM_MOVETO branch, into `Class36.method269`.
        // Height in 1/128-tile units, scaled by the client as for CAM_LOOKAT.
        #[cfg(before_742)]
        {
            buf.p1_alt1(self.local_x);
            buf.p1_alt2(self.local_z);
            buf.p1_alt1(self.rate2);
            buf.p2(self.height);
            buf.p1_alt1(self.rate);
        }
        #[cfg(since_742)]
        {
            buf.p1_alt2(self.local_x);
            buf.p1_alt3(self.local_z);
            buf.p2_alt1(self.height);
            buf.p1(self.rate);
            buf.p1_alt3(self.rate2);
        }
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.local_x)
            + size_of_val(&self.local_z)
            + size_of_val(&self.height)
            + size_of_val(&self.rate)
            + size_of_val(&self.rate2)
    }
}
