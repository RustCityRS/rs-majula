use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[cfg(before_530)]
#[server_prot(CamShake, Buffered, Fixed)]
pub struct CamShake {
    pub axis: u8,
    pub ran: u8,
    pub amp: u8,
    pub rate: u8,
}

#[cfg(before_530)]
impl ServerProtMessage for CamShake {
    fn encode(&self, buf: &mut Packet) {
        buf.p1(self.axis);
        buf.p1(self.ran);
        buf.p1(self.amp);
        buf.p1(self.rate);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.axis)
            + size_of_val(&self.ran)
            + size_of_val(&self.amp)
            + size_of_val(&self.rate)
    }
}

#[cfg(since_530)]
#[server_prot(CamShake, Buffered, Fixed)]
pub struct CamShake {
    pub axis: u8,
    pub ran: u8,
    pub amp: u8,
    pub rate: u8,
    /// The wobble's starting phase.
    pub cycle: u16,
    #[cfg(all(since_530, before_647))]
    pub verify: u16,
}

#[cfg(all(since_530, before_647))]
impl ServerProtMessage for CamShake {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.verify);
        buf.p1(self.axis);
        buf.p1(self.ran);
        buf.p1(self.amp);
        buf.p1(self.rate);
        buf.p2(self.cycle);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.axis)
            + size_of_val(&self.ran)
            + size_of_val(&self.amp)
            + size_of_val(&self.rate)
            + size_of_val(&self.cycle)
            + size_of_val(&self.verify)
    }
}

#[cfg(since_647)]
impl ServerProtMessage for CamShake {
    #[cfg(before_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt2(self.ran);
        buf.p1(self.amp);
        buf.p2_alt3(self.cycle);
        buf.p1(self.axis);
        buf.p1_alt1(self.rate);
    }

    #[cfg(since_742)]
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt1(self.ran);
        buf.p1_alt2(self.amp);
        buf.p1_alt2(self.rate);
        buf.p2_alt2(self.cycle);
        buf.p1_alt2(self.axis);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.axis)
            + size_of_val(&self.ran)
            + size_of_val(&self.amp)
            + size_of_val(&self.rate)
            + size_of_val(&self.cycle)
    }
}
