use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_647)]
#[server_prot(CamShake, Buffered, Fixed)]
pub struct CamShake {
    pub direction: u8,
    pub jitter: u8,
    pub amplitude: u8,
    pub frequency: u8,
}

#[cfg(before_647)]
impl ServerProtMessage for CamShake {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.direction);
        buf.p1(self.jitter);
        buf.p1(self.amplitude);
        buf.p1(self.frequency);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.direction)
            + size_of_val(&self.jitter)
            + size_of_val(&self.amplitude)
            + size_of_val(&self.frequency)
    }
}

#[cfg(since_647)]
#[server_prot(CamShake, Buffered, Fixed)]
pub struct CamShake {
    pub direction: u8,
    pub jitter: u8,
    pub amplitude: u8,
    pub frequency: u8,
    /// The wobble's starting phase.
    pub cycle: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for CamShake {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.jitter);
        buf.p1(self.amplitude);
        buf.p2_alt3(self.cycle);
        buf.p1(self.direction);
        buf.p1_alt1(self.frequency);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.direction)
            + size_of_val(&self.jitter)
            + size_of_val(&self.amplitude)
            + size_of_val(&self.frequency)
            + size_of_val(&self.cycle)
    }
}
