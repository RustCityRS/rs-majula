#[cfg(since_647)]
use crate::network::game::info::hit::Hit;
#[cfg(since_742)]
use crate::network::game::info::hit::{HEALTH_BAR_SIZE, encode_health_bar};
use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

#[cfg(before_647)]
pub struct PlayerInfoDamage {
    pub damage: u8,
    pub damage_type: u8,
    pub current: u8,
    pub base: u8,
}

#[cfg(before_647)]
impl InfoMessage for PlayerInfoDamage {
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
pub struct PlayerInfoDamage<'a> {
    pub hits: &'a [Hit],
    pub bright: u8,
}

#[cfg(all(since_647, before_742))]
impl InfoMessage for PlayerInfoDamage<'_> {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p1(self.hits.len() as u8);
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

#[cfg(since_742)]
impl InfoMessage for PlayerInfoDamage<'_> {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p1_alt1(self.hits.len() as u8);
        for (i, hit) in self.hits.iter().enumerate() {
            hit.encode(buf, self.bright >> i & 1 != 0, Packet::p1_alt3);
        }
        buf.p1_alt1(self.hits.len().min(1) as u8);
        if let Some(last) = self.hits.last() {
            encode_health_bar(buf, last.health, Packet::p1);
        }
    }

    fn sizeof(&self) -> usize {
        2 + self
            .hits
            .iter()
            .enumerate()
            .map(|(i, hit)| hit.sizeof(self.bright >> i & 1 != 0))
            .sum::<usize>()
            + self.hits.len().min(1) * HEALTH_BAR_SIZE
    }
}
