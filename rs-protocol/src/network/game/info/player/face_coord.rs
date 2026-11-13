use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

/// The client's angles: 16384 units to the turn, so this many to the radian.
#[cfg(since_647)]
const UNITS_PER_RADIAN: f64 = 2607.5945876176133;
#[cfg(since_647)]
const ANGLE_MASK: i32 = 0x3FFF;

#[derive(Clone, Copy, Default)]
pub struct PlayerInfoFaceCoord {
    pub x: u16,
    pub z: u16,
    #[cfg(since_647)]
    pub from_x: u16,
    #[cfg(since_647)]
    pub from_z: u16,
}

#[cfg(before_647)]
impl InfoMessage for PlayerInfoFaceCoord {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2(self.x);
        buf.p2(self.z);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z)
    }
}

#[cfg(since_647)]
impl InfoMessage for PlayerInfoFaceCoord {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2_alt2(self.angle());
    }

    fn sizeof(&self) -> usize {
        size_of::<u16>()
    }
}

#[cfg(since_647)]
impl PlayerInfoFaceCoord {
    /// The way the client turns a player at `from_x`, `from_z` to face `x`,
    /// `z` (fine coords): an angle in 16384ths of a turn, 0 facing south.
    fn angle(&self) -> u16 {
        let dx = self.from_x as i32 - self.x as i32;
        let dz = self.from_z as i32 - self.z as i32;
        if dx == 0 && dz == 0 {
            return 0;
        }
        let angle = (f64::atan2(dx as f64, dz as f64) * UNITS_PER_RADIAN) as i32;
        (angle & ANGLE_MASK) as u16
    }
}
