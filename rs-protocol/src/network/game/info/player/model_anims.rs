use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

/// The bytes of one entry: its seq, delay and slots.
const ENTRY: usize = 5;

#[derive(Clone, Copy)]
pub struct ModelAnim {
    pub seq: u16,
    pub delay: u8,
    pub slots: u16,
}

pub struct PlayerInfoModelAnims<'a> {
    pub anims: &'a [ModelAnim],
}

impl InfoMessage for PlayerInfoModelAnims<'_> {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p1_alt2(self.anims.len() as u8);
        for anim in self.anims {
            buf.p2_alt1(anim.seq);
            buf.p1_alt1(anim.delay);
            buf.p2(anim.slots);
        }
    }

    fn sizeof(&self) -> usize {
        1 + self.anims.len() * ENTRY
    }
}
