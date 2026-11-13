use rs_io::Packet;

pub trait InfoMessage {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>);
    fn sizeof(&self) -> usize;
}
