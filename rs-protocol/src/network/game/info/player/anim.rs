use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

pub struct PlayerInfoAnim {
    pub seq: u16,
    pub delay: u8,
}

#[cfg(before_647)]
impl InfoMessage for PlayerInfoAnim {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2(self.seq);
        buf.p1(self.delay);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.seq) + size_of_val(&self.delay)
    }
}

#[cfg(since_647)]
impl InfoMessage for PlayerInfoAnim {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        for _ in 0..4 {
            buf.p2(self.seq);
        }
        buf.p1_alt1(self.delay);
    }

    fn sizeof(&self) -> usize {
        4 * size_of_val(&self.seq) + size_of_val(&self.delay)
    }
}
