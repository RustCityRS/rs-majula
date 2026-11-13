use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

/// Type 0 and label 15: the spotanim isn't attached to a model point.
#[cfg(since_647)]
const UNATTACHED: u8 = 15 << 3;

pub struct PlayerInfoSpotAnim {
    pub spotanim: u16,
    pub height: u16,
    pub delay: u16,
}

#[cfg(before_647)]
impl InfoMessage for PlayerInfoSpotAnim {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2(self.spotanim);
        buf.p4(((self.height as i32) << 16) | self.delay as i32);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.spotanim) + size_of_val(&self.height) + size_of_val(&self.delay)
    }
}

#[cfg(since_647)]
impl InfoMessage for PlayerInfoSpotAnim {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2_alt2(self.spotanim);
        buf.p4_alt3(((self.height as i32) << 16) | self.delay as i32);
        buf.p1_alt3(UNATTACHED);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.spotanim) + size_of_val(&self.height) + size_of_val(&self.delay) + 1
    }
}
