#[cfg(since_742)]
use crate::network::game::info::smart2or4null_size;
use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;
use rs_ver::TypeId;

pub struct NpcInfoAnim {
    pub seq: TypeId,
    pub delay: u8,
}

#[cfg(before_647)]
impl InfoMessage for NpcInfoAnim {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2(self.seq.g2());
        buf.p1(self.delay);
    }

    fn sizeof(&self) -> usize {
        size_of::<u16>() + size_of_val(&self.delay)
    }
}

#[cfg(all(since_647, before_742))]
impl InfoMessage for NpcInfoAnim {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        for _ in 0..4 {
            buf.p2_alt2(self.seq.g2());
        }
        buf.p1_alt2(self.delay);
    }

    fn sizeof(&self) -> usize {
        4 * size_of::<u16>() + size_of_val(&self.delay)
    }
}

#[cfg(since_742)]
impl InfoMessage for NpcInfoAnim {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        let seq = self.seq.int();
        for _ in 0..4 {
            buf.psmartint2(seq);
        }
        buf.p1(self.delay);
    }

    fn sizeof(&self) -> usize {
        4 * smart2or4null_size(self.seq.int()) + size_of_val(&self.delay)
    }
}
