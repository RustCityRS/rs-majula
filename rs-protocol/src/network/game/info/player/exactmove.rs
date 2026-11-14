use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

/// The client's angle for each script direction (north, east, south, west,
/// then the diagonals from northeast), 0 facing south.
#[cfg(since_742)]
const ANGLES: [u16; 8] = [8192, 12288, 0, 4096, 10240, 14336, 2048, 6144];

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

#[cfg(all(since_647, before_742))]
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

#[cfg(since_742)]
impl InfoMessage for PlayerInfoExactMove {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p1_alt3(self.start_x);
        buf.p1(self.start_z);
        buf.p1_alt2(self.end_x);
        buf.p1(self.end_z);
        buf.p2_alt2(self.begin);
        buf.p2_alt1(self.finish);
        buf.p2(ANGLES[(self.dir & 7) as usize]);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.start_x)
            + size_of_val(&self.start_z)
            + size_of_val(&self.end_x)
            + size_of_val(&self.end_z)
            + size_of_val(&self.begin)
            + size_of_val(&self.finish)
            + size_of::<u16>()
    }
}
