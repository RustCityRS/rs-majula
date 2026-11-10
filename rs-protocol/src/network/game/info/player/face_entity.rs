use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

pub struct PlayerInfoFaceEntity {
    pub entity: u16,
}

#[cfg(before_647)]
impl InfoMessage for PlayerInfoFaceEntity {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2(self.entity);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.entity)
    }
}

#[cfg(since_647)]
impl InfoMessage for PlayerInfoFaceEntity {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2_alt3(self.entity);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.entity)
    }
}
