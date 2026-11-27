use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

pub struct PlayerInfoMoveSpeed {
    pub move_speed: u8,
}

impl InfoMessage for PlayerInfoMoveSpeed {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        #[cfg(before_742)]
        buf.p1(self.move_speed);
        #[cfg(since_742)]
        buf.p1_alt1(self.move_speed);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.move_speed)
    }
}
