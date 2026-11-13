use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

#[derive(Clone, Copy, Default)]
pub struct NpcInfoFaceCoord {
    pub x: u16,
    pub z: u16,
}

#[cfg(before_647)]
impl InfoMessage for NpcInfoFaceCoord {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2(self.x);
        buf.p2(self.z);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z)
    }
}

#[cfg(since_647)]
impl InfoMessage for NpcInfoFaceCoord {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2(self.x);
        buf.p2_alt1(self.z);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.x) + size_of_val(&self.z)
    }
}
