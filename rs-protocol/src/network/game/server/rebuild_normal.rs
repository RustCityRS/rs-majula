use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
#[cfg(since_647)]
use rs_io::packet::BitWriter;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
#[cfg(rev = "225")]
use std::collections::{HashMap, HashSet};

#[cfg(rev = "225")]
#[server_prot(RebuildNormal, Immediate, VarShort)]
pub struct RebuildNormal {
    pub zone_x: u16,
    pub zone_z: u16,
    pub mapsquares: HashSet<u16>,
    pub crcs: HashMap<(char, u8, u8), i32>,
}

#[cfg(rev = "225")]
impl ServerProtMessage for RebuildNormal {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.zone_x);
        buf.p2(self.zone_z);
        for mapsquare in &self.mapsquares {
            let x = (mapsquare >> 8) as u8;
            let z = (mapsquare & 0xFF) as u8;
            buf.p1(x);
            buf.p1(z);
            match self.crcs.get(&('m', x, z)).copied() {
                None => buf.p4(0),
                Some(crc) => buf.p4(crc),
            }
            match self.crcs.get(&('l', x, z)).copied() {
                None => buf.p4(0),
                Some(crc) => buf.p4(crc),
            }
        }
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.zone_x) + size_of_val(&self.zone_z) + (self.mapsquares.len() * 10)
    }
}

#[cfg(all(since_244, before_647))]
#[server_prot(RebuildNormal, Immediate, Fixed)]
pub struct RebuildNormal {
    pub zone_x: u16,
    pub zone_z: u16,
}

#[cfg(all(since_244, before_647))]
impl ServerProtMessage for RebuildNormal {
    fn encode(&self, buf: &mut Packet) {
        buf.p2(self.zone_x);
        buf.p2(self.zone_z);
    }

    fn sizeof(&self) -> usize {
        size_of_val(&self.zone_x) + size_of_val(&self.zone_z)
    }
}

/// Maximum number of player slots (matches xlite-2 client). Slot 0 is
/// reserved; valid player indices are `1..MAX_PLAYERS`.
#[cfg(since_647)]
pub const MAX_PLAYERS: usize = 2048;

#[cfg(since_647)]
#[server_prot(RebuildNormal, Immediate, VarShort)]
pub struct RebuildNormal {
    /// Local player's slot index (used for the login PlayerInfo init block).
    pub self_index: u16,
    pub player_x: u16,
    pub player_z: u16,
    pub player_plane: u8,
    /// If true, prepend the PlayerInfo init bit-stream (login only).
    pub login: bool,
    /// Last-known 18-bit packed region (`level << 16 | x/64 << 8 | z/64`)
    /// for each player slot. Length is expected to be [`MAX_PLAYERS`].
    /// Slot 0 and the self slot are skipped by the encoder. Empty slots
    /// use `0`. Only read when `login` is true — mirrors Void's
    /// `playerRegions: IntArray` input to `mapInit`.
    pub player_regions: Vec<u32>,
    /// XTEA key blocks for every mapsquare covering the build area, in
    /// the order the client expects. Looked up by the engine.
    pub xtea_keys: Vec<[i32; 4]>,
}

#[cfg(since_647)]
impl ServerProtMessage for RebuildNormal {
    fn encode(&self, buf: &mut Packet) {
        let chunk_x = self.player_x >> 3;
        let chunk_z = self.player_z >> 3;

        // PlayerInfo init bit-stream (login only) — bit-packing crosses
        // byte boundaries, so build into a local scratch buffer and
        // splice via `pdata`.
        if self.login {
            let other_count = self
                .player_regions
                .len()
                .saturating_sub(1) // self
                .saturating_sub(1); // slot 0
            let total_bits = 30 + other_count * 18;
            let bit_bytes = (total_bits + 7) >> 3;
            let mut bits_pkt = Packet::new(bit_bytes);
            let mut writer = BitWriter::new();

            let player_pos = (self.player_z as u32 & 0x3FFF)
                | ((self.player_x as u32 & 0x3FFF) << 14)
                | ((self.player_plane as u32 & 0x3) << 28);
            writer.pbit::<30>(&mut bits_pkt, player_pos as i32);

            let self_index = self.self_index as usize;
            for (i, &region) in self.player_regions.iter().enumerate() {
                if i == 0 || i == self_index {
                    continue;
                }
                writer.pbit::<18>(&mut bits_pkt, region as i32);
            }
            writer.finish(&mut bits_pkt);

            buf.pdata(&bits_pkt.data, 0, bit_bytes);
        }

        // Header
        buf.p2_alt1(chunk_x);
        let login_val: u8 = if self.login { 1 } else { 0 };
        buf.p1_alt2(login_val);
        buf.p1_alt2(0); // build_area_idx
        buf.p2_alt3(chunk_z);

        // XTEA keys (4 × i32 BE per mapsquare)
        for keys in &self.xtea_keys {
            for &k in keys {
                buf.p4(k);
            }
        }
    }

    fn sizeof(&self) -> usize {
        // 6-byte header + 16 bytes per mapsquare's XTEA key block.
        let xtea_bytes = self.xtea_keys.len() * 16;
        let header = 6;
        let init_bits = if self.login {
            // 30 bits self + 18 bits per other slot.
            let other_count = self.player_regions.len().saturating_sub(2);
            (30 + other_count * 18 + 7) >> 3
        } else {
            0
        };
        init_bits + header + xtea_bytes
    }
}
