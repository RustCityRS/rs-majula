use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

#[derive(Default)]
pub struct PlayerInfoExactMove {
    pub start_x: u8,
    pub start_z: u8,
    pub end_x: u8,
    pub end_z: u8,
    pub begin: u16,
    pub finish: u16,
    pub dir: u8,
}

#[cfg(before_647)]
impl InfoMessage for PlayerInfoExactMove {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p1(self.start_x);
        buf.p1(self.start_z);
        buf.p1(self.end_x);
        buf.p1(self.end_z);
        buf.p2(self.begin);
        buf.p2(self.finish);
        buf.p1(self.dir);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.start_x)
            + size_of_val(&self.start_z)
            + size_of_val(&self.end_x)
            + size_of_val(&self.end_z)
            + size_of_val(&self.begin)
            + size_of_val(&self.finish)
            + size_of_val(&self.dir)
    }
}

#[cfg(since_647)]
impl InfoMessage for PlayerInfoExactMove {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p1_alt3(self.start_x);
        buf.p1(self.start_z);
        buf.p1_alt2(self.end_x);
        buf.p1_alt3(self.end_z);
        buf.p2_alt2(self.begin);
        buf.p2(self.finish);
        buf.p1_alt2(self.dir);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.start_x)
            + size_of_val(&self.start_z)
            + size_of_val(&self.end_x)
            + size_of_val(&self.end_z)
            + size_of_val(&self.begin)
            + size_of_val(&self.finish)
            + size_of_val(&self.dir)
    }
}
