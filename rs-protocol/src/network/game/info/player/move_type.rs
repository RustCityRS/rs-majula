use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

pub struct PlayerInfoMoveType {
    pub move_type: u8,
}

impl InfoMessage for PlayerInfoMoveType {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p1(self.move_type);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.move_type)
    }
}
