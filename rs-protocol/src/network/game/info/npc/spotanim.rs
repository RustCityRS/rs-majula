use crate::network::game::info_prot_message::InfoMessage;
use rs_io::Packet;

/// Type 0 and label 15: the spotanim isn't attached to a model point.
#[cfg(since_647)]
const UNATTACHED: u8 = 15 << 3;

pub struct NpcInfoSpotAnim {
    pub spotanim: u16,
    pub height: u16,
    pub delay: u16,
}

#[cfg(before_647)]
impl InfoMessage for NpcInfoSpotAnim {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2(self.spotanim);
        buf.p4(((self.height as i32) << 16) | self.delay as i32);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.spotanim) + size_of_val(&self.height) + size_of_val(&self.delay)
    }
}

#[cfg(since_647)]
impl InfoMessage for NpcInfoSpotAnim {
    fn encode<T: AsRef<[u8]> + AsMut<[u8]>>(&self, buf: &mut Packet<T>) {
        buf.p2_alt3(self.spotanim);
        buf.p4_alt3(((self.height as i32) << 16) | self.delay as i32);
        buf.p1_alt1(UNATTACHED);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.spotanim) + size_of_val(&self.height) + size_of_val(&self.delay) + 1
    }
}

#[cfg(all(test, since_647))]
mod tests {
    use super::*;

    #[test]
    fn npc_spotanim_reads_back_through_the_client() {
        let mut buf = Packet::from([0u8; 7]);
        NpcInfoSpotAnim {
            spotanim: 74,
            height: 92,
            delay: 0,
        }
        .encode(&mut buf);
        let b = buf.data;
        assert_eq!(b, [202, 0, 92, 0, 0, 0, 248], "exact spotanim wire bytes");
        assert_eq!(((b[1] as u16) << 8) | b[0].wrapping_sub(128) as u16, 74);
        let g4 =
            ((b[2] as u32) << 16) | ((b[3] as u32) << 24) | (b[4] as u32) | ((b[5] as u32) << 8);
        assert_eq!((g4 >> 16, g4 & 0xFFFF), (92, 0), "height, delay");
        let g1 = b[6].wrapping_sub(128);
        assert_eq!(
            (g1 & 0x7, (g1 >> 3) & 0xF),
            (0, 15),
            "type 0, no attachment"
        );
    }
}
