//! Rev-647 mask-block renderer.
//!
//! Same cache-and-write architecture as the jag-era renderer: the info phase
//! calls `compute_info` once per entity to encode that tick's mask blocks
//! into per-entity slots, and the per-observer info encoders copy the cached
//! bytes with `write`. The engine keeps its compact u16 `info_prot` mask
//! language on [`EntityMasks`]; this module owns the 647 wire mask values
//! ([`PlayerInfoProt647`] / [`NpcInfoProt647`], u32 with extension bytes)
//! and translates between the two at encode time.
//!
//! Wire shapes from the deob-647 client (Class244.method1590 for players,
//! Statics1.method3939 for NPCs).

use crate::{EntityMasks, MASK_MOVE_TYPE, MASK_TEMP_MOVE_TYPE};
use rs_grid::CoordGrid;
use rs_io::Packet;
use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};
use std::ops::{Index, IndexMut};

fn face_angle(src_x: i32, src_z: i32, dst_x: i32, dst_z: i32) -> u16 {
    let dx = src_x - dst_x;
    let dz = src_z - dst_z;
    if dx == 0 && dz == 0 {
        return 0;
    }
    let angle = (f64::atan2(dx as f64, dz as f64) * 2607.5945876176133) as i32;
    (angle & 0x3FFF) as u16
}

const MAX_PLAYERS: usize = 2048;
// Matches rs-entity's rev-647 MAX_NPCS (15-bit npc ids in the add format).
const MAX_NPCS: usize = 32768;
const PLAYER_PROT_COUNT: usize = 9;
// One slot per fixed-size NPC block: Hit, FaceEntity, FaceCoord, SpotAnim,
// ChangeType, Anim. Say is variable-length and kept apart.
const NPC_PROT_COUNT: usize = 6;

/// Rev-647 player info wire mask bits (Class244.method1590).
/// Extension: 0x20 signals byte1 follows, 0x8000 signals byte2 follows.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerInfoProt647 {
    FaceCoord = 0x1,
    Hit = 0x2,
    Appearance = 0x4,
    MoveType = 0x8,
    FaceEntity = 0x10,
    // 0x20 = MASK_EXTENDED (not a prot)
    /// Spotanim channel 0. 0x2000 and 0x10000 are channels 1 and 2.
    SpotAnim = 0x40,
    Anim = 0x80,
    // 0x100 is a timer bar, not chat: 647 public chat is the MESSAGE_PUBLIC
    // packet, and the jag-era Chat block has no 647 counterpart.
    Say = 0x200,
    TempMoveType = 0x800,
    ExactMove = 0x4000,
}

pub const PLAYER_MASK_EXTENDED: u32 = 0x20;
pub const PLAYER_MASK_EXTENDED2: u32 = 0x8000;

impl PlayerInfoProt647 {
    pub const fn to_index(self) -> usize {
        match self {
            Self::FaceCoord => 0,
            Self::Appearance => 1,
            Self::MoveType => 2,
            Self::FaceEntity => 3,
            Self::Anim => 4,
            Self::TempMoveType => 5,
            Self::ExactMove => 6,
            Self::Hit => 7,
            Self::SpotAnim => 8,
            // Variable-length; kept outside the fixed slots.
            Self::Say => 0,
        }
    }
}

/// Rev-647 NPC info wire mask bits (Statics1.method3939).
/// Extension: 0x08 in byte0 signals byte1 follows.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcInfoProt647 {
    Hit = 0x1,
    ChangeType = 0x2,
    FaceCoord = 0x4,
    // 0x8 = MASK_EXTENDED
    FaceEntity = 0x10,
    /// Spotanim channel 0. 0x1000 and 0x20000 are channels 1 and 2.
    SpotAnim = 0x20,
    Say = 0x40,
    Anim = 0x80,
    // 0x100 = MASK_EXTENDED2
}

pub const NPC_MASK_EXTENDED: u32 = 0x08;
pub const NPC_MASK_EXTENDED2: u32 = 0x100;

impl NpcInfoProt647 {
    pub const fn to_index(self) -> usize {
        match self {
            Self::Hit => 0,
            Self::FaceEntity => 1,
            Self::FaceCoord => 2,
            Self::SpotAnim => 3,
            Self::ChangeType => 4,
            Self::Anim => 5,
            // Variable-length; kept outside the fixed slots.
            Self::Say => 0,
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
struct Slot {
    // Room for the largest fixed block: a hits block carrying both of an
    // entity's hits, 1 + 2 x (three smarts + a byte) = 15 bytes.
    data: [u8; 16],
    len: u8,
}

/// The fixed-size blocks of every entity, one row per block kind, in one heap
/// allocation. A `Box::new([[Slot; N]; R])` would lay the whole table out on
/// the stack first -- megabytes, for npcs.
struct SlotTable {
    slots: Box<[Slot]>,
    width: usize,
}

impl SlotTable {
    fn new(rows: usize, width: usize) -> Self {
        Self {
            slots: vec![Slot::EMPTY; rows * width].into_boxed_slice(),
            width,
        }
    }

    fn rows_mut(&mut self) -> impl Iterator<Item = &mut [Slot]> {
        self.slots.chunks_mut(self.width)
    }
}

impl Index<usize> for SlotTable {
    type Output = [Slot];

    fn index(&self, row: usize) -> &[Slot] {
        &self.slots[row * self.width..(row + 1) * self.width]
    }
}

impl IndexMut<usize> for SlotTable {
    fn index_mut(&mut self, row: usize) -> &mut [Slot] {
        &mut self.slots[row * self.width..(row + 1) * self.width]
    }
}

impl Slot {
    const EMPTY: Self = Self {
        data: [0; 16],
        len: 0,
    };

    #[inline(always)]
    fn is_set(&self) -> bool {
        self.len > 0
    }

    #[inline(always)]
    fn write_to(&self, buf: &mut Packet) {
        buf.pdata(&self.data[..self.len as usize], 0, self.len as usize);
    }

    #[inline(always)]
    fn set_p1(&mut self, a: u8) {
        self.data[0] = a;
        self.len = 1;
    }

    /// p2_alt1: (hi, lo + 128)
    #[inline(always)]
    fn set_p2_alt1(&mut self, a: u16) {
        self.data[0] = (a >> 8) as u8;
        self.data[1] = (a as u8).wrapping_add(128);
        self.len = 2;
    }

    /// ip2_alt1: (lo + 128, hi) — little-endian with add
    #[inline(always)]
    fn set_ip2_alt1(&mut self, a: u16) {
        self.data[0] = (a as u8).wrapping_add(128);
        self.data[1] = (a >> 8) as u8;
        self.len = 2;
    }

    /// ip2: (lo, hi) — little-endian, no offset. Matches the client's `ig2()`.
    #[inline(always)]
    fn set_ip2(&mut self, a: u16) {
        self.data[0] = a as u8;
        self.data[1] = (a >> 8) as u8;
        self.len = 2;
    }

    /// p1_alt1: (0 - value) — negate
    #[inline(always)]
    fn set_p1_alt1(&mut self, a: u8) {
        self.data[0] = 0u8.wrapping_sub(a);
        self.len = 1;
    }

    /// 647 exactmove: p1b_alt2(start_x) + p1b(start_z) + p1b_alt1(end_x) +
    /// p1b_alt2(end_z) + p2_alt1(begin) + p2(finish) + p1_alt1(dir) = 9 bytes
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn set_exactmove_647(
        &mut self,
        start_x: i8,
        start_z: i8,
        end_x: i8,
        end_z: i8,
        begin: u16,
        finish: u16,
        dir: u8,
    ) {
        // p1b_alt2: 128 - value
        self.data[0] = 128u8.wrapping_sub(start_x as u8);
        // p1b: raw signed byte
        self.data[1] = start_z as u8;
        // p1b_alt1: 0 - value
        self.data[2] = 0u8.wrapping_sub(end_x as u8);
        // p1b_alt2: 128 - value
        self.data[3] = 128u8.wrapping_sub(end_z as u8);
        // p2_alt1: [hi, lo+128]
        self.data[4] = (begin >> 8) as u8;
        self.data[5] = (begin as u8).wrapping_add(128);
        // p2: big-endian
        let fb = finish.to_be_bytes();
        self.data[6] = fb[0];
        self.data[7] = fb[1];
        // p1_alt1: 0 - value
        self.data[8] = 0u8.wrapping_sub(dir);
        self.len = 9;
    }

    /// gSmart1or2-compatible write at `pos`: 1 byte if 0..=127, else 2 bytes
    /// big-endian of (value + 32768). Returns the number of bytes written.
    #[inline(always)]
    fn put_smart(&mut self, pos: usize, value: i32) -> usize {
        if (0..=127).contains(&value) {
            self.data[pos] = value as u8;
            1
        } else {
            let v = (value + 32768) as u16;
            self.data[pos] = (v >> 8) as u8;
            self.data[pos + 1] = v as u8;
            2
        }
    }

    /// 647 hits block: a count, then per hit `psmart(hitmark) +
    /// psmart(damage) + psmart(delay) + p1(health_percent)`. The trailing byte
    /// is the overhead health-bar fill (`current * 255 / base`, 0-255). The
    /// count's byte transform differs by entity, so the caller encodes it.
    /// Returns bytes written.
    #[inline(always)]
    fn set_hits(&mut self, count: u8, hits: &[Hit]) -> u8 {
        let mut n = 0;
        self.data[n] = count;
        n += 1;
        for hit in hits {
            n += self.put_smart(n, hit.hitmark);
            n += self.put_smart(n, hit.damage);
            n += self.put_smart(n, 0); // delay
            self.data[n] = hit.health_percent;
            n += 1;
        }
        self.len = n as u8;
        self.len
    }

    /// 647 player animation, mask 0x80: the seq for each of the four
    /// movement-speed channels as `p2`, then `p1_alt3(delay)` -- the client
    /// reads `g1_alt3` (`Class244.method1590`). 9 bytes.
    #[inline(always)]
    fn set_player_anim_647(&mut self, anim: u16, delay: u8) {
        let ab = anim.to_be_bytes();
        for channel in 0..4 {
            self.data[channel * 2] = ab[0];
            self.data[channel * 2 + 1] = ab[1];
        }
        self.data[8] = delay.wrapping_add(128);
        self.len = 9;
    }

    /// 647 npc animation, mask 0x80: four `p2_alt1` seqs, then
    /// `p1_alt1(delay)` (`Statics1.method3939`). 9 bytes.
    #[inline(always)]
    fn set_npc_anim_647(&mut self, anim: u16, delay: u8) {
        for channel in 0..4 {
            self.data[channel * 2] = (anim >> 8) as u8;
            self.data[channel * 2 + 1] = (anim as u8).wrapping_add(128);
        }
        self.data[8] = 0u8.wrapping_sub(delay);
        self.len = 9;
    }

    /// p2: big-endian. Matches the client's `g2()`.
    #[inline(always)]
    fn set_p2(&mut self, a: u16) {
        self.data[..2].copy_from_slice(&a.to_be_bytes());
        self.len = 2;
    }

    /// 647 player spotanim channel 0, mask 0x40:
    /// `p2_alt1(id) + p4_alt1(height<<16 | delay) + p1_alt2((label<<3)|type)`
    /// = 7 bytes, with `label = -1` (15) and `type = 0`, as the npc block.
    #[inline(always)]
    fn set_player_spotanim_647(&mut self, id: u16, height: u16, delay: u16) {
        self.data[0] = (id >> 8) as u8;
        self.data[1] = (id as u8).wrapping_add(128);
        let packed: u32 = ((height as u32) << 16) | (delay as u32);
        self.data[2] = (packed >> 16) as u8;
        self.data[3] = (packed >> 24) as u8;
        self.data[4] = packed as u8;
        self.data[5] = (packed >> 8) as u8;
        self.data[6] = 128u8.wrapping_sub(15 << 3);
        self.len = 7;
    }

    /// 647 NPC spotanim (graphic) channel 0 — mask 0x20:
    /// `ip2_alt1(id) + p4_alt1(height<<16 | delay) + p1_alt3((label<<3)|type)`
    /// = 7 bytes, with `label = -1` (15) and `type = 0` (no attachment).
    #[inline(always)]
    fn set_npc_spotanim_647(&mut self, id: u16, height: u16, delay: u16) -> u8 {
        // ip2_alt1: (lo + 128, hi)
        self.data[0] = (id as u8).wrapping_add(128);
        self.data[1] = (id >> 8) as u8;
        // p4_alt1: byte order [v>>16, v>>24, v, v>>8]
        let packed: u32 = ((height as u32) << 16) | (delay as u32);
        self.data[2] = (packed >> 16) as u8;
        self.data[3] = (packed >> 24) as u8;
        self.data[4] = packed as u8;
        self.data[5] = (packed >> 8) as u8;
        // p1_alt3: (value + 128); value = (label<<3)|type, label=-1→15, type=0
        let g1: u8 = 15 << 3;
        self.data[6] = g1.wrapping_add(128);
        self.len = 7;
        7
    }
}

/// One hit in a 647 hits block.
#[derive(Clone, Copy)]
struct Hit {
    hitmark: i32,
    damage: i32,
    health_percent: u8,
}

/// Overhead health-bar fill for a hit: `current * 255 / base`, 0-255.
fn health_percent(current: Option<u8>, base: Option<u8>) -> u8 {
    match (current, base) {
        (Some(current), Some(base)) if base > 0 => {
            ((current as u32 * 255) / (base as u32)).min(255) as u8
        }
        _ => 0,
    }
}

/// This tick's hits: the first and, when the second slot is set, the second.
/// 225/289 send each in its own block; 647 carries both in one, with a count.
///
/// The hitmark is the jag-era damage type as the engine holds it. 647 has its
/// own hitmark set -- `void-634`'s `HitSplat.Mark` and the 786 hitmark
/// configs show melee/range/magic/regular at 0-3 and a miss at 8 -- and the
/// mapping belongs with the combat port.
fn hits(info: &EntityMasks, first: bool, second: bool) -> ([Hit; 2], usize) {
    let mut out = [Hit {
        hitmark: 0,
        damage: 0,
        health_percent: 0,
    }; 2];
    let mut n = 0;
    if first {
        out[n] = Hit {
            hitmark: info.damage_type.unwrap_or(0) as i32,
            damage: info.damage_taken.unwrap_or(0) as i32,
            health_percent: health_percent(info.damage_current, info.damage_base),
        };
        n += 1;
    }
    if second {
        out[n] = Hit {
            hitmark: info.damage2_type.unwrap_or(0) as i32,
            damage: info.damage2_taken.unwrap_or(0) as i32,
            health_percent: health_percent(info.damage2_current, info.damage2_base),
        };
        n += 1;
    }
    (out, n)
}

pub struct PlayerRenderer {
    fixed: SlotTable,
    appearances: Vec<Option<Vec<u8>>>,
    says: Vec<Option<Vec<u8>>>,
    highs: Box<[u16; MAX_PLAYERS]>,
    lows: Box<[u16; MAX_PLAYERS]>,
    /// Each player's stored move type, kept past the tick it was sent in so
    /// an observer that adds the player later is told it too.
    move_types: Box<[u8; MAX_PLAYERS]>,
}

/// What the client assumes until told otherwise (`AClass5_Sub1.method2354`).
const DEFAULT_MOVE_TYPE: u8 = 1;

impl PlayerRenderer {
    #[inline]
    pub fn new() -> PlayerRenderer {
        PlayerRenderer {
            fixed: SlotTable::new(PLAYER_PROT_COUNT, MAX_PLAYERS),
            appearances: vec![None; MAX_PLAYERS],
            says: vec![None; MAX_PLAYERS],
            highs: Box::new([0; MAX_PLAYERS]),
            lows: Box::new([0; MAX_PLAYERS]),
            move_types: Box::new([DEFAULT_MOVE_TYPE; MAX_PLAYERS]),
        }
    }

    /// Encode this tick's mask blocks for one player. `coord` is the
    /// player's current tile — the 647 FaceCoord block carries a precomputed
    /// angle from it to the face target.
    #[inline]
    pub fn compute_info(&mut self, pid: usize, info: &EntityMasks, coord: CoordGrid) {
        let masks: u16 = info.masks;

        if masks == 0 {
            return;
        }

        let mut highs: u16 = 0;
        let mut lows: u16 = 0;
        let mut wire: u32 = 0;

        if masks & PlayerInfoProt::FaceCoord as u16 != 0 {
            wire |= PlayerInfoProt647::FaceCoord as u32;
            let src_x = CoordGrid::fine(coord.x(), 1) as i32;
            let src_z = CoordGrid::fine(coord.z(), 1) as i32;
            let (dst_x, dst_z) = if let (Some(fx), Some(fz)) = (info.face_x, info.face_z) {
                (fx as i32, fz as i32)
            } else {
                (src_x, src_z)
            };
            let angle = face_angle(src_x, src_z, dst_x, dst_z);
            self.fixed[PlayerInfoProt647::FaceCoord.to_index()][pid].set_p2_alt1(angle);
            highs += 2;
            lows += 2;
        }
        if masks & PlayerInfoProt::Appearance as u16 != 0 {
            wire |= PlayerInfoProt647::Appearance as u32;
            // The appearance block already includes its length prefix.
            if let Some(bytes) = info.last_appearance_info.as_ref() {
                let len = bytes.len();
                let slot = &mut self.appearances[pid];
                match slot {
                    Some(v) => {
                        v.clear();
                        v.extend_from_slice(bytes);
                    }
                    None => {
                        *slot = Some(bytes.to_vec());
                    }
                }
                highs += len as u16;
                lows += len as u16;
            }
        }
        if masks & MASK_MOVE_TYPE != 0 {
            wire |= PlayerInfoProt647::MoveType as u32;
            let move_type = info.move_type.unwrap_or(0);
            self.move_types[pid] = move_type;
            self.fixed[PlayerInfoProt647::MoveType.to_index()][pid].set_p1(move_type);
            highs += 1;
        }
        if masks & PlayerInfoProt::FaceEntity as u16 != 0 {
            wire |= PlayerInfoProt647::FaceEntity as u32;
            // 647: ip2_alt1(entity) — 2 bytes
            self.fixed[PlayerInfoProt647::FaceEntity.to_index()][pid]
                .set_ip2_alt1(info.face_entity.unwrap_or(u16::MAX));
            highs += 2;
            lows += 2;
        }
        if masks & PlayerInfoProt::Anim as u16 != 0 {
            wire |= PlayerInfoProt647::Anim as u32;
            self.fixed[PlayerInfoProt647::Anim.to_index()][pid].set_player_anim_647(
                info.anim_id.unwrap_or(u16::MAX),
                info.anim_delay.unwrap_or(0),
            );
            highs += 9;
        }
        if masks & PlayerInfoProt::SpotAnim as u16 != 0 {
            wire |= PlayerInfoProt647::SpotAnim as u32;
            self.fixed[PlayerInfoProt647::SpotAnim.to_index()][pid].set_player_spotanim_647(
                info.spotanim.unwrap_or(u16::MAX),
                info.spotanim_height.unwrap_or(0),
                info.spotanim_delay.unwrap_or(0),
            );
            highs += 7;
        }
        if masks & PlayerInfoProt::Say as u16 != 0 {
            wire |= PlayerInfoProt647::Say as u32;
            // 647: gjstr (null-terminated string)
            if let Some(say) = info.say.as_ref() {
                let bytes = say.as_bytes();
                let len = bytes.len() + 1;
                let slot = &mut self.says[pid];
                match slot {
                    Some(v) => {
                        v.clear();
                        v.extend_from_slice(bytes);
                        v.push(0);
                    }
                    None => {
                        let mut buf = Vec::with_capacity(len);
                        buf.extend_from_slice(bytes);
                        buf.push(0);
                        *slot = Some(buf);
                    }
                }
                highs += len as u16;
            }
        }
        // The jag-era Chat mask (0x40) is NOT used for public chat in rev-647:
        // public chat goes through the MESSAGE_PUBLIC server packet.
        if masks & MASK_TEMP_MOVE_TYPE != 0 {
            wire |= PlayerInfoProt647::TempMoveType as u32;
            self.fixed[PlayerInfoProt647::TempMoveType.to_index()][pid]
                .set_p1_alt1(info.temp_move_type.unwrap_or(0));
            highs += 1;
        }
        if masks & PlayerInfoProt::ExactMove as u16 != 0 {
            wire |= PlayerInfoProt647::ExactMove as u32;
            self.fixed[PlayerInfoProt647::ExactMove.to_index()][pid].set_exactmove_647(
                info.exactmove_start_x.unwrap_or(0) as i8,
                info.exactmove_start_z.unwrap_or(0) as i8,
                info.exactmove_end_x.unwrap_or(0) as i8,
                info.exactmove_end_z.unwrap_or(0) as i8,
                info.exactmove_begin.unwrap_or(0),
                info.exactmove_finish.unwrap_or(0),
                info.exactmove_dir.unwrap_or(0),
            );
            highs += 9;
        }
        let first = masks & PlayerInfoProt::Damage as u16 != 0;
        let second = masks & PlayerInfoProt::Damage2 as u16 != 0;
        if first || second {
            wire |= PlayerInfoProt647::Hit as u32;
            let (hits, count) = hits(info, first, second);
            // The player count is a plain `g1`.
            let len = self.fixed[PlayerInfoProt647::Hit.to_index()][pid]
                .set_hits(count as u8, &hits[..count]);
            highs += len as u16;
        }

        if highs > 0 {
            self.highs[pid] = highs + Self::header(wire);
        }

        if lows > 0 {
            let low_masks = PlayerInfoProt647::Appearance as u32
                | PlayerInfoProt647::FaceEntity as u32
                | PlayerInfoProt647::FaceCoord as u32
                | PlayerInfoProt647::MoveType as u32;
            let header = Self::header(low_masks);
            let appearance_len = self.appearances[pid].as_ref().map_or(0, |v| v.len()) as u16;
            // face_entity(2) + face_coord(2) + move_type(1)
            self.lows[pid] = header + appearance_len + 2 + 2 + 1;
        }
    }

    #[inline(always)]
    pub fn write(&self, buf: &mut Packet, id: u16, prot: PlayerInfoProt647) {
        let idx = id as usize;
        match prot {
            PlayerInfoProt647::Appearance => {
                if let Some(bytes) = &self.appearances[idx] {
                    buf.pdata(bytes, 0, bytes.len());
                }
            }
            PlayerInfoProt647::Say => {
                if let Some(bytes) = &self.says[idx] {
                    buf.pdata(bytes, 0, bytes.len());
                }
            }
            _ => {
                self.fixed[prot.to_index()][idx].write_to(buf);
            }
        }
    }

    #[inline(always)]
    pub fn has(&self, id: u16, prot: PlayerInfoProt647) -> bool {
        match prot {
            PlayerInfoProt647::Appearance => self.appearances[id as usize]
                .as_ref()
                .is_some_and(|v| !v.is_empty()),
            PlayerInfoProt647::Say => self.says[id as usize]
                .as_ref()
                .is_some_and(|v| !v.is_empty()),
            _ => self.fixed[prot.to_index()][id as usize].is_set(),
        }
    }

    #[inline(always)]
    pub fn cache_face_entity(&mut self, id: u16, entity: u16) {
        self.fixed[PlayerInfoProt647::FaceEntity.to_index()][id as usize].set_ip2_alt1(entity);
    }

    /// Cache a FaceCoord block (precomputed 647 angle).
    #[inline(always)]
    pub fn cache_face_coord(&mut self, id: u16, angle: u16) {
        self.fixed[PlayerInfoProt647::FaceCoord.to_index()][id as usize].set_p2_alt1(angle);
    }

    /// Cache a MoveType block holding the player's stored move type, for an
    /// observer adding them. The client plays an added player's steps at
    /// whatever its slot last held -- another player's, if it was reused.
    #[inline(always)]
    pub fn cache_move_type(&mut self, id: u16) {
        let move_type = self.move_types[id as usize];
        self.fixed[PlayerInfoProt647::MoveType.to_index()][id as usize].set_p1(move_type);
    }

    #[inline(always)]
    pub const fn highdefinitions(&self, id: u16) -> usize {
        self.highs[id as usize] as usize
    }

    #[inline(always)]
    pub const fn lowdefinitions(&self, id: u16) -> usize {
        self.lows[id as usize] as usize
    }

    #[inline]
    pub fn remove_temporary(&mut self, active: &[u16]) {
        for &pid in active {
            let idx = pid as usize;
            self.highs[idx] = 0;
            self.fixed[PlayerInfoProt647::Anim.to_index()][idx] = Slot::EMPTY;
            self.fixed[PlayerInfoProt647::FaceEntity.to_index()][idx] = Slot::EMPTY;
            self.fixed[PlayerInfoProt647::FaceCoord.to_index()][idx] = Slot::EMPTY;
            self.fixed[PlayerInfoProt647::MoveType.to_index()][idx] = Slot::EMPTY;
            self.fixed[PlayerInfoProt647::TempMoveType.to_index()][idx] = Slot::EMPTY;
            self.fixed[PlayerInfoProt647::ExactMove.to_index()][idx] = Slot::EMPTY;
            self.fixed[PlayerInfoProt647::Hit.to_index()][idx] = Slot::EMPTY;
            self.fixed[PlayerInfoProt647::SpotAnim.to_index()][idx] = Slot::EMPTY;
            if let Some(v) = &mut self.says[idx] {
                v.clear();
            }
        }
    }

    #[inline]
    pub fn remove_permanent(&mut self, id: u16) {
        let idx = id as usize;
        self.highs[idx] = 0;
        self.lows[idx] = 0;
        self.appearances[idx] = None;
        self.move_types[idx] = DEFAULT_MOVE_TYPE;
    }

    #[inline(always)]
    fn header(masks: u32) -> u16 {
        // 647: 0x20 extension for byte1, 0x8000 extension for byte2
        if masks > 0xffff {
            3
        } else if masks > 0xff {
            2
        } else {
            1
        }
    }
}

pub struct NpcRenderer {
    fixed: SlotTable,
    highs: Box<[u16; MAX_NPCS]>,
    lows: Box<[u16; MAX_NPCS]>,
    says: Vec<Option<Vec<u8>>>,
}

impl NpcRenderer {
    #[inline]
    pub fn new() -> NpcRenderer {
        NpcRenderer {
            fixed: SlotTable::new(NPC_PROT_COUNT, MAX_NPCS),
            highs: Box::new([0; MAX_NPCS]),
            lows: Box::new([0; MAX_NPCS]),
            says: vec![None; MAX_NPCS],
        }
    }

    #[inline]
    pub fn compute_info(&mut self, nid: usize, info: &EntityMasks) {
        let masks: u16 = info.masks;

        if masks == 0 {
            return;
        }

        let mut highs: u16 = 0;
        let mut wire: u32 = 0;

        if masks & NpcInfoProt::ChangeType as u16 != 0 {
            wire |= NpcInfoProt647::ChangeType as u32;
            self.fixed[NpcInfoProt647::ChangeType.to_index()][nid]
                .set_p2(info.changetype.unwrap_or(u16::MAX));
            highs += 2;
        }
        if masks & NpcInfoProt::FaceCoord as u16 != 0
            && let (Some(face_x), Some(face_z)) = (info.face_x, info.face_z)
        {
            wire |= NpcInfoProt647::FaceCoord as u32;
            self.cache_face_coord(nid as u16, face_x, face_z);
            highs += 4;
        }
        if masks & NpcInfoProt::Anim as u16 != 0 {
            wire |= NpcInfoProt647::Anim as u32;
            self.fixed[NpcInfoProt647::Anim.to_index()][nid].set_npc_anim_647(
                info.anim_id.unwrap_or(u16::MAX),
                info.anim_delay.unwrap_or(0),
            );
            highs += 9;
        }
        let first = masks & NpcInfoProt::Damage as u16 != 0;
        let second = masks & NpcInfoProt::Damage2 as u16 != 0;
        if first || second {
            wire |= NpcInfoProt647::Hit as u32;
            let (hits, count) = hits(info, first, second);
            // The npc count is `g1_alt3`.
            let len = self.fixed[NpcInfoProt647::Hit.to_index()][nid]
                .set_hits((count as u8).wrapping_add(128), &hits[..count]);
            highs += len as u16;
        }
        if masks & NpcInfoProt::FaceEntity as u16 != 0 {
            wire |= NpcInfoProt647::FaceEntity as u32;
            // The 647 client reads NPC FaceEntity via ig2() — little-endian,
            // no offset (players use ig2_alt1 = LE+128).
            self.fixed[NpcInfoProt647::FaceEntity.to_index()][nid]
                .set_ip2(info.face_entity.unwrap_or(u16::MAX));
            highs += 2;
        }
        if masks & NpcInfoProt::SpotAnim as u16 != 0 {
            wire |= NpcInfoProt647::SpotAnim as u32;
            let len = self.fixed[NpcInfoProt647::SpotAnim.to_index()][nid].set_npc_spotanim_647(
                info.spotanim.unwrap_or(u16::MAX),
                info.spotanim_height.unwrap_or(0),
                info.spotanim_delay.unwrap_or(0),
            );
            highs += len as u16;
        }
        if masks & NpcInfoProt::Say as u16 != 0 {
            wire |= NpcInfoProt647::Say as u32;
            if let Some(say) = info.say.as_ref() {
                let bytes = say.as_bytes();
                let len = bytes.len() + 1;
                let slot = &mut self.says[nid];
                match slot {
                    Some(v) => {
                        v.clear();
                        v.extend_from_slice(bytes);
                        v.push(0);
                    }
                    None => {
                        let mut buf = Vec::with_capacity(len);
                        buf.extend_from_slice(bytes);
                        buf.push(0);
                        *slot = Some(buf);
                    }
                }
                highs += len as u16;
            }
        }

        if highs > 0 {
            self.highs[nid] = highs + Self::header(wire);
        }
    }

    #[inline(always)]
    pub fn write(&self, buf: &mut Packet, id: u16, prot: NpcInfoProt647) {
        match prot {
            NpcInfoProt647::Say => {
                if let Some(bytes) = &self.says[id as usize] {
                    buf.pdata(bytes, 0, bytes.len());
                }
            }
            _ => {
                self.fixed[prot.to_index()][id as usize].write_to(buf);
            }
        }
    }

    #[inline(always)]
    pub fn has(&self, id: u16, prot: NpcInfoProt647) -> bool {
        match prot {
            NpcInfoProt647::Say => self.says[id as usize]
                .as_ref()
                .is_some_and(|v| !v.is_empty()),
            _ => self.fixed[prot.to_index()][id as usize].is_set(),
        }
    }

    #[inline(always)]
    pub fn cache_face_entity(&mut self, id: u16, entity: u16) {
        // little-endian (ig2), matching the per-tick FaceEntity write above
        self.fixed[NpcInfoProt647::FaceEntity.to_index()][id as usize].set_ip2(entity);
    }

    /// Cache a FaceCoord block: p2(fine_x) + ip2(fine_z), 4 bytes.
    #[inline(always)]
    pub fn cache_face_coord(&mut self, id: u16, x: u16, z: u16) {
        let slot = &mut self.fixed[NpcInfoProt647::FaceCoord.to_index()][id as usize];
        let xb = x.to_be_bytes();
        let zb = z.to_le_bytes();
        slot.data[0] = xb[0];
        slot.data[1] = xb[1];
        slot.data[2] = zb[0];
        slot.data[3] = zb[1];
        slot.len = 4;
    }

    #[inline(always)]
    pub const fn highdefinitions(&self, id: u16) -> usize {
        self.highs[id as usize] as usize
    }

    #[inline(always)]
    pub const fn lowdefinitions(&self, id: u16) -> usize {
        self.lows[id as usize] as usize
    }

    #[inline]
    pub fn remove_temporary(&mut self, active: &[u16]) {
        for &nid in active {
            let idx = nid as usize;
            self.highs[idx] = 0;
            for slot_arr in self.fixed.rows_mut() {
                slot_arr[idx] = Slot::EMPTY;
            }
            if let Some(v) = &mut self.says[idx] {
                v.clear();
            }
        }
    }

    #[inline]
    pub fn remove_permanent(&mut self, id: u16) {
        let idx = id as usize;
        self.highs[idx] = 0;
        self.lows[idx] = 0;
        self.says[idx] = None;
    }

    #[inline(always)]
    fn header(masks: u32) -> u16 {
        if masks > 0xffff {
            3
        } else if masks > 0xff {
            2
        } else {
            1
        }
    }
}

#[cfg(test)]
mod move_type_tests {
    use super::*;

    fn move_type_block(r: &PlayerRenderer, pid: u16) -> Option<u8> {
        r.has(pid, PlayerInfoProt647::MoveType)
            .then(|| r.fixed[PlayerInfoProt647::MoveType.to_index()][pid as usize].data[0])
    }

    /// A player's move type goes out once, when it changes. An observer that
    /// adds them ticks later must still be told it, so the renderer keeps it
    /// after the tick's block is cleared.
    #[test]
    fn an_added_player_carries_the_move_type_sent_ticks_earlier() {
        let mut r = PlayerRenderer::new();
        let mut info = EntityMasks::new();
        info.masks = MASK_MOVE_TYPE;
        info.move_type = Some(2);
        r.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        assert_eq!(move_type_block(&r, 7), Some(2));

        r.remove_temporary(&[7]);
        assert_eq!(move_type_block(&r, 7), None, "the tick's block is gone");

        r.cache_move_type(7);
        assert_eq!(
            move_type_block(&r, 7),
            Some(2),
            "and the add still says run"
        );
    }

    #[test]
    fn a_slot_freed_by_logout_goes_back_to_walk() {
        let mut r = PlayerRenderer::new();
        let mut info = EntityMasks::new();
        info.masks = MASK_MOVE_TYPE;
        info.move_type = Some(2);
        r.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        r.remove_temporary(&[7]);
        r.remove_permanent(7);
        r.cache_move_type(7);
        assert_eq!(move_type_block(&r, 7), Some(DEFAULT_MOVE_TYPE));
    }
}

#[cfg(test)]
mod npc_spotanim_tests {
    use super::*;

    /// The 647 NPC spotanim (mask 0x20) encoder must produce bytes the
    /// client's `Statics1.method3939` reads back exactly — `ig2_alt1(id) +
    /// g4_alt1(height<<16|delay) + g1_alt3((label<<3)|type)`.
    #[test]
    fn npc_spotanim_647_roundtrips_through_client_decode() {
        let mut slot = Slot::EMPTY;
        let len = slot.set_npc_spotanim_647(74, 92, 0);
        assert_eq!(len, 7, "spotanim block is 7 bytes (2+4+1)");

        let b = &slot.data[..7];
        assert_eq!(b, &[202, 0, 92, 0, 0, 0, 248], "exact spotanim wire bytes");

        // ig2_alt1: lo = byte0 - 128, hi = byte1  → id = (hi<<8)|lo
        let lo = b[0].wrapping_sub(128) as u16;
        let hi = b[1] as u16;
        assert_eq!((hi << 8) | lo, 74, "id round-trips");

        // g4_alt1: value = (b2<<16)|(b3<<24)|b4|(b5<<8)
        let g4 =
            ((b[2] as u32) << 16) | ((b[3] as u32) << 24) | (b[4] as u32) | ((b[5] as u32) << 8);
        assert_eq!(g4 >> 16, 92, "height = g4 >> 16");
        assert_eq!(g4 & 0xFFFF, 0, "delay = g4 & 0xFFFF");

        // g1_alt3: value = byte6 - 128; type = value & 0x7; label = (value>>3)&0xF
        let g1 = b[6].wrapping_sub(128);
        assert_eq!(g1 & 0x7, 0, "type = 0");
        assert_eq!((g1 >> 3) & 0xF, 15, "label = 15 (no attachment)");
    }
}
