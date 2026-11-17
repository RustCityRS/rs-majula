use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

pub struct PlayerInfoChat<'a> {
    pub colour: u8,
    pub effects: u8,
    pub crown: u8,
    pub bytes: &'a [u8],
}

#[cfg(any(before_530, since_647))]
impl InfoMessage for PlayerInfoChat<'_> {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p1(self.colour);
        buf.p1(self.effects);
        buf.p1(self.crown);
        #[cfg(before_647)]
        buf.p1(self.bytes.len() as u8);
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.colour)
            + size_of_val(&self.effects)
            + size_of_val(&self.crown)
            + cfg!(before_647) as usize
            + self.bytes.len()
    }
}

#[cfg(all(since_530, before_647))]
impl InfoMessage for PlayerInfoChat<'_> {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2_alt1(((self.colour as u16) << 8) | self.effects as u16);
        buf.p1(self.crown);
        buf.p1(self.bytes.len() as u8);
        buf.pdata_alt1(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.colour)
            + size_of_val(&self.effects)
            + size_of_val(&self.crown)
            + 1
            + self.bytes.len()
    }
}
