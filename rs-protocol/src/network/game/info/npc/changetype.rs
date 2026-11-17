#[cfg(since_742)]
use crate::network::game::info::smart2or4null_size;
use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct NpcInfoChangeType {
    pub ntype: TypeId,
}

#[cfg(any(before_530, all(since_647, before_742)))]
impl InfoMessage for NpcInfoChangeType {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2(self.ntype.g2());
    }

    fn sizeof(&self) -> usize {
        size_of::<u16>()
    }
}

#[cfg(since_742)]
impl InfoMessage for NpcInfoChangeType {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.psmartint2(self.ntype.int());
    }

    fn sizeof(&self) -> usize {
        smart2or4null_size(self.ntype.int())
    }
}

#[cfg(all(since_530, before_647))]
impl InfoMessage for NpcInfoChangeType {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2_alt1(self.ntype.g2());
    }

    fn sizeof(&self) -> usize {
        size_of::<u16>()
    }
}
