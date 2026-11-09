use crate::network::game::server::ServerProtMessage;
#[cfg(since_647)]
use crate::network::game::server::player_info::player_info_init;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
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

#[cfg(since_647)]
#[server_prot(RebuildNormal, Immediate, VarShort)]
pub struct RebuildNormal {
    pub self_index: u16,
    pub player_x: u16,
    pub player_z: u16,
    pub player_plane: u8,
    pub login: bool,
    pub player_regions: Vec<u32>,
    pub xtea_keys: Vec<[i32; 4]>,
}

#[cfg(since_647)]
impl ServerProtMessage for RebuildNormal {
    fn encode(&self, buf: &mut Packet) {
        if self.login {
            let init = player_info_init(
                self.self_index,
                self.player_x,
                self.player_z,
                self.player_plane,
                &self.player_regions,
            );
            buf.pdata(&init.data, 0, init.pos);
        }
        buf.p2_alt1(self.player_x >> 3);
        buf.p1_alt2(self.login as u8);
        buf.p1_alt2(0);
        buf.p2_alt3(self.player_z >> 3);
        for keys in &self.xtea_keys {
            for &k in keys {
                buf.p4(k);
            }
        }
    }

    fn sizeof(&self) -> usize {
        let init = if self.login {
            (30 + 18 * self.player_regions.len().saturating_sub(2) + 7) >> 3
        } else {
            0
        };
        init + 2 + 1 + 1 + 2 + self.xtea_keys.len() * 16
    }
}
