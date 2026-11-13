use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

pub struct NpcInfoChangeType {
    pub ntype: u16,
}

impl InfoMessage for NpcInfoChangeType {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2(self.ntype);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.ntype)
    }
}
