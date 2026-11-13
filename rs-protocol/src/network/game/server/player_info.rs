use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::packet::BitWriter;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;

#[server_prot(PlayerInfo, Immediate, VarShort)]
pub struct PlayerInfo<'a> {
    pub bytes: &'a [u8],
}

impl ServerProtMessage for PlayerInfo<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        self.bytes.len()
    }
}

/// The `PLAYER_INFO` state a client starts from, read with its first
/// `REBUILD_NORMAL` and on a reconnect: its own tile, then the region it
/// keeps for every other slot.
#[cfg(since_647)]
pub fn player_info_init(pid: u16, x: u16, z: u16, level: u8, regions: &[u32]) -> Packet {
    let mut buf = Packet::new((30 + 18 * regions.len().saturating_sub(2) + 7) >> 3);
    let mut bits = BitWriter::new();
    bits.pbit::<30>(
        &mut buf,
        ((level as i32 & 0x3) << 28) | ((x as i32 & 0x3FFF) << 14) | (z as i32 & 0x3FFF),
    );
    for (i, &region) in regions.iter().enumerate() {
        if i != 0 && i != pid as usize {
            bits.pbit::<18>(&mut buf, region as i32);
        }
    }
    bits.finish(&mut buf);
    buf
}
