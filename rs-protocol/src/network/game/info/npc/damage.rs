#[cfg(since_647)]
use crate::network::game::info::hit::Hit;
use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

#[cfg(before_647)]
pub struct NpcInfoDamage {
    pub damage: u8,
    pub damage_type: u8,
    pub current: u8,
    pub base: u8,
}

#[cfg(before_647)]
impl InfoMessage for NpcInfoDamage {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p1(self.damage);
        buf.p1(self.damage_type);
        buf.p1(self.current);
        buf.p1(self.base);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.damage)
            + size_of_val(&self.damage_type)
            + size_of_val(&self.current)
            + size_of_val(&self.base)
    }
}

#[cfg(since_647)]
pub struct NpcInfoDamage<'a> {
    pub hits: &'a [Hit],
    pub bright: u8,
}

#[cfg(since_647)]
impl InfoMessage for NpcInfoDamage<'_> {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p1_alt1(self.hits.len() as u8);
        for (i, hit) in self.hits.iter().enumerate() {
            hit.encode(buf, self.bright >> i & 1 != 0);
        }
    }

    fn sizeof(&self) -> usize {
        1 + self
            .hits
            .iter()
            .enumerate()
            .map(|(i, hit)| hit.sizeof(self.bright >> i & 1 != 0))
            .sum::<usize>()
    }
}
