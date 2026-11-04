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

// `CamShake` — shake the camera for a period.
// Rev-647 opcode 96 (CAM_SHAKE), fixed 6.
// Layout:
//   `[p1_alt3 intensity][p1_alt1 shake_type][p2_alt1 cycle][p1_alt2 movement][p1_alt1 speed]`.

/// Rev-647 CAM_SHAKE (opcode 96, fixed 6): the same four parameters as
/// before 647, plus the shake's starting cycle.
///
/// The client (`MapAreaConfig`'s CAM_SHAKE branch, then `Class237`) offsets
/// the camera axis `shake_type` each frame by
/// `random(-jitter..=jitter) + sin(frequency / 100 * cycle) * amplitude`, and
/// counts `cycle` up from what this packet sends.
#[cfg(since_647)]
#[server_prot(CamShake, Buffered, Fixed)]
pub struct CamShake {
    /// Which camera axis shakes (the client keeps five).
    pub shake_type: u8,
    pub jitter: u8,
    pub amplitude: u8,
    pub frequency: u8,
    /// The wobble's starting phase.
    pub cycle: u16,
}

#[cfg(since_647)]
impl ServerProtMessage for CamShake {
    fn encode(&self, buf: &mut Packet) {
        buf.p1_alt1(self.jitter);
        buf.p1(self.amplitude);
        buf.ip2_alt1(self.cycle);
        buf.p1(self.shake_type);
        buf.p1_alt3(self.frequency);
    }

    fn sizeof(&self) -> usize {
        6
    }
}
