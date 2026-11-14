use crate::network::game::server::ServerProtMessage;
use crate::network::game::server_prot::ServerProt;
use crate::network::game::server_prot_message::ServerProtMessageInfo;
use crate::network::game::server_prot_priority::ServerProtPriority;
use rs_io::{Packet, PacketFrame};
use rs_protocol_macros::server_prot;
#[cfg(since_530)]
use std::f64::consts::TAU;

/// Each octant's facing, from south (angle 0) round through west: on 530 the
/// index into `sg.f` (NW, N, NE, W, E, SW, S, SE), on 647 the client's
/// `(direction + 4) << 11`.
#[cfg(all(since_530, before_647))]
const DIRECTIONS: [u8; 8] = [6, 5, 3, 0, 1, 2, 4, 7];
#[cfg(since_647)]
const DIRECTIONS: [u8; 8] = [4, 5, 6, 7, 0, 1, 2, 3];

#[server_prot(NpcInfo, Immediate, VarShort)]
pub struct NpcInfo<'a> {
    pub bytes: &'a [u8],
}

impl ServerProtMessage for NpcInfo<'_> {
    fn encode(&self, buf: &mut Packet) {
        buf.pdata(self.bytes, 0, self.bytes.len());
    }

    fn sizeof(&self) -> usize {
        self.bytes.len()
    }
}

/// The 3-bit facing of an NPC_INFO add block (on 530 also a PLAYER_INFO add)
/// for an entity facing `(dx, dz)` away from it, in fine coords (entity minus
/// the coord it faces).
///
/// The 647 client turns the field into an angle as `(direction + 4) << 11`
/// and sets it outright (`SteroSoundClientOption`,
/// `PathingEntity.method4004(angle, true)`); a FaceCoord, by contrast, only
/// turns an npc whose BAS can turn (`method3996`), so this is the facing such
/// an npc keeps. The 530 client sets the angle `sg.f` holds at the field's
/// index on an entity it creates (`oi.a`, `oc.b`). Its angle for a face coord
/// is `atan2(dx, dz)` in 16384 units, 0 facing south. With no facing
/// (`None`), angle 0, the client's own default (`NpcType.spawn_direction`).
#[cfg(since_530)]
pub fn add_direction(facing: Option<(i32, i32)>) -> u8 {
    let Some((dx, dz)) = facing.filter(|&d| d != (0, 0)) else {
        return DIRECTIONS[0];
    };
    // 16384 units to the turn: the client's 2607.5945876176133.
    let angle = ((dx as f64).atan2(dz as f64) * (16384.0 / TAU)) as i32 & 0x3FFF;
    DIRECTIONS[(((angle + 1024) >> 11) & 7) as usize]
}

#[cfg(all(test, since_647))]
mod tests {
    use super::add_direction;

    /// The client's angle `(direction + 4) << 11`, 0 facing south
    /// (`Class20_Sub1.anIntArray355`). `(dx, dz)` is npc minus face coord.
    #[test]
    fn add_direction_is_the_client_octant_of_the_way_it_faces() {
        assert_eq!(add_direction(Some((0, 2))), 4, "south: angle 0");
        assert_eq!(add_direction(Some((0, -2))), 0, "north: angle 8192");
        assert_eq!(add_direction(Some((2, 0))), 6, "west: angle 4096");
        assert_eq!(add_direction(Some((-2, 0))), 2, "east: angle 12288");
        assert_eq!(add_direction(Some((2, 2))), 5, "south-west: angle 2048");
    }

    /// With no facing, the client's own default: 4, angle 0.
    #[test]
    fn add_direction_defaults_to_the_client_s_spawn_direction() {
        assert_eq!(add_direction(None), 4);
        assert_eq!(add_direction(Some((0, 0))), 4);
    }
}
