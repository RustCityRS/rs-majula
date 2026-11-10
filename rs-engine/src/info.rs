use crate::active_npc::ActiveNpc;
use crate::active_player::ActivePlayer;
use crate::build::ActiveBuildArea;
#[cfg(before_647)]
use crate::core::OtherPlayers;
use crate::core::engine;
use crate::watch::Target;
use rs_entity::{BuildArea, MAX_NPCS, MAX_PLAYERS, Npc, PathingEntity, Player};
#[cfg(since_647)]
use rs_entity::{MoveSpeed, dir_delta, face_dir};
use rs_grid::CoordGrid;
use rs_info::Visibility;
use rs_info::{NpcRenderer, Observer, PlayerRenderer};
use rs_io::Packet;
use rs_io::packet::BitWriter;
use rs_protocol::network::game::MAX_PACKET_SIZE;
use rs_zone::zone_map::ZoneMap;
use std::mem;
use std::ops::Deref;

/// The most an info packet's body holds: its opcode and length take 3 bytes.
const BYTES_LIMIT: usize = MAX_PACKET_SIZE - 3;

/// Whether `bits` more bits and `bytes` more block bytes still fit.
#[inline(always)]
fn room(bw: &BitWriter, updates: &Packet, bits: usize, bytes: usize) -> bool {
    ((bw.bitpos() + bits + 7) >> 3) + updates.pos + bytes <= BYTES_LIMIT
}

/// What every observer gets of a player this tick, indexed by pid: its
/// update block's length, the flags its removal reads, and its movement as
/// one code of `bits` bits (none when it didn't move). The code is worked
/// out once and copied into each packet that has the player.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct PlayerSnapshot {
    pub coord: u32,
    pub len: u16,
    pub bits: u8,
    pub flags: u8,
    pub code: u64,
}

impl PlayerSnapshot {
    pub const PRESENT: u8 = 1 << 0;
    pub const ACTIVE: u8 = 1 << 1;
    pub const TELE: u8 = 1 << 2;
    pub const VIS_HARD: u8 = 1 << 3;
    pub const JUMP: u8 = 1 << 4;

    /// The "no live player here" entry the snapshot is reset to each tick.
    pub const ABSENT: PlayerSnapshot = PlayerSnapshot {
        coord: 0,
        len: 0,
        bits: 0,
        flags: 0,
        code: 0,
    };

    /// Player `p` with an update block of `len` bytes.
    pub fn new(p: &Player, len: u16) -> PlayerSnapshot {
        let mut flags = PlayerSnapshot::PRESENT;
        if p.active {
            flags |= PlayerSnapshot::ACTIVE;
        }
        if p.pathing.tele {
            flags |= PlayerSnapshot::TELE;
        }
        if p.info.vis == Visibility::Hard {
            flags |= PlayerSnapshot::VIS_HARD;
        }
        if p.pathing.jump {
            flags |= PlayerSnapshot::JUMP;
        }
        let (code, bits) = Self::movement(&p.pathing);
        PlayerSnapshot {
            coord: p.pathing.coord.packed(),
            len,
            bits,
            flags,
            code,
        }
    }

    #[cfg(before_647)]
    fn movement(pathing: &PathingEntity) -> (u64, u8) {
        let (code, bits) = steps(pathing.walk_dir, pathing.run_dir);
        (code as u64, bits)
    }

    /// The move from the tile every client has the player on, last tick's:
    /// a teleport within 15 tiles or further, a run, or a walk.
    #[cfg(since_647)]
    fn movement(pathing: &PathingEntity) -> (u64, u8) {
        let (coord, last) = (pathing.coord, pathing.last_coord);
        if pathing.tele {
            let dx = coord.x() as i64 - last.x() as i64;
            let dz = coord.z() as i64 - last.z() as i64;
            let dl = coord.y() as i64 - last.y() as i64;
            if dx.abs() <= 15 && dz.abs() <= 15 {
                // 2 + 1 + 2 + 5 + 5 = 15 bits
                let code = (3 << 13) | ((dl & 0x3) << 10) | ((dx & 0x1F) << 5) | (dz & 0x1F);
                (code as u64, 15)
            } else {
                // 2 + 1 + 2 + 14 + 14 = 33 bits
                let code = (3 << 31)
                    | (1 << 30)
                    | ((dl & 0x3) << 28)
                    | ((dx & 0x3FFF) << 14)
                    | (dz & 0x3FFF);
                (code as u64, 33)
            }
        } else if coord == last {
            (0, 0)
        } else if pathing.run_dir != -1 {
            let (wx, wz) = dir_delta(pathing.walk_dir);
            let (rx, rz) = dir_delta(pathing.run_dir);
            let (dx, dz) = (wx as i32 + rx as i32, wz as i32 + rz as i32);
            match run_index(dx, dz) {
                // 2 + 4 = 6 bits
                Some(ri) => (((2 << 4) | ri) as u64, 6),
                // 2 + 3 = 5 bits
                None => (
                    ((1 << 3) | WALK_DIR_MAP[face_dir(0, 0, dx, dz).max(0) as usize]) as u64,
                    5,
                ),
            }
        } else {
            (
                ((1 << 3) | WALK_DIR_MAP[pathing.walk_dir as usize]) as u64,
                5,
            )
        }
    }

    /// Clears the `PRESENT` bit, mirroring a `players[pid].take()` / removal so
    /// later observers in the same tick encode a remove rather than movement.
    #[inline(always)]
    pub const fn clear(&mut self) {
        self.flags = 0;
    }

    /// Whether the tracked player should be removed from the observer's view
    /// this tick: gone, teleported, off the observer's level or view
    /// distance, inactive, or hard-hidden.
    #[inline(always)]
    pub fn should_remove(&self, obs_coord: CoordGrid, obs_y: u8, view_distance: u8) -> bool {
        self.flags & PlayerSnapshot::PRESENT == 0
            || self.flags & PlayerSnapshot::TELE != 0
            || CoordGrid::from(self.coord).y() != obs_y
            || !CoordGrid::in_distance(&obs_coord, CoordGrid::from(self.coord), view_distance)
            || self.flags & PlayerSnapshot::ACTIVE == 0
            || self.flags & PlayerSnapshot::VIS_HARD != 0
    }
}

/// Every slot's [`PlayerSnapshot`] this tick, with where each player can be
/// seen from laid out 64 slots to a row, so an observer's view of all 2048
/// slots is a run of vector compares.
pub struct PlayerSnapshots {
    all: Box<[PlayerSnapshot; MAX_PLAYERS]>,
    xs: Box<[[u16; 64]; MAX_PLAYERS >> 6]>,
    zs: Box<[[u16; 64]; MAX_PLAYERS >> 6]>,
    levels: Box<[[u8; 64]; MAX_PLAYERS >> 6]>,
    #[cfg(since_647)]
    changed: Box<[u64; MAX_PLAYERS >> 6]>,
}

impl PlayerSnapshots {
    /// The level of a slot nobody can see: empty, and on 647 also inactive
    /// or hard-hidden (before it, the add pass checks those itself).
    const UNSEEN: u8 = u8::MAX;
    #[cfg(before_647)]
    const SEEN: u8 = PlayerSnapshot::PRESENT;
    #[cfg(since_647)]
    const SEEN: u8 = PlayerSnapshot::PRESENT | PlayerSnapshot::ACTIVE;
    #[cfg(before_647)]
    const SEEN_MASK: u8 = PlayerSnapshot::PRESENT;
    #[cfg(since_647)]
    const SEEN_MASK: u8 = Self::SEEN | PlayerSnapshot::VIS_HARD;

    pub fn new() -> PlayerSnapshots {
        PlayerSnapshots {
            all: Box::new([PlayerSnapshot::ABSENT; MAX_PLAYERS]),
            xs: Box::new([[0; 64]; MAX_PLAYERS >> 6]),
            zs: Box::new([[0; 64]; MAX_PLAYERS >> 6]),
            levels: Box::new([[Self::UNSEEN; 64]; MAX_PLAYERS >> 6]),
            #[cfg(since_647)]
            changed: Box::new([0; MAX_PLAYERS >> 6]),
        }
    }

    /// Every slot empty, for a new tick.
    pub fn reset(&mut self) {
        self.all.fill(PlayerSnapshot::ABSENT);
        self.levels.fill([Self::UNSEEN; 64]);
        #[cfg(since_647)]
        self.changed.fill(0);
    }

    pub fn set(&mut self, pid: u16, s: PlayerSnapshot) {
        let (w, b) = ((pid >> 6) as usize, (pid & 63) as usize);
        let coord = CoordGrid::from(s.coord);
        self.all[pid as usize] = s;
        self.xs[w][b] = coord.x();
        self.zs[w][b] = coord.z();
        self.levels[w][b] = if s.flags & Self::SEEN_MASK == Self::SEEN {
            coord.y()
        } else {
            Self::UNSEEN
        };
        #[cfg(since_647)]
        {
            let changed = (s.bits != 0 || s.len != 0) as u64;
            self.changed[w] = self.changed[w] & !(1 << b) | changed << b;
        }
    }

    /// Player `pid` left mid-tick: the observers still to come remove it.
    pub fn clear(&mut self, pid: u16) {
        self.all[pid as usize].clear();
        self.levels[(pid >> 6) as usize][(pid & 63) as usize] = Self::UNSEEN;
    }

    /// The slots an observer at `coord` sees, `distance` tiles each way on
    /// its level, one bit each. Plain indexes over fixed rows, so it stays
    /// vectorized in builds that don't inline iterator adapters.
    #[allow(clippy::needless_range_loop)]
    pub fn view(&self, coord: CoordGrid, distance: u8, out: &mut [u64; MAX_PLAYERS >> 6]) {
        let d = distance as u16;
        let (x0, z0, span, y) = (
            coord.x().wrapping_sub(d),
            coord.z().wrapping_sub(d),
            d << 1,
            coord.y(),
        );
        for w in 0..MAX_PLAYERS >> 6 {
            let (xs, zs, levels) = (&self.xs[w], &self.zs[w], &self.levels[w]);
            let mut seen = [0u8; 64];
            for j in 0..64 {
                seen[j] = ((xs[j].wrapping_sub(x0) <= span)
                    & (zs[j].wrapping_sub(z0) <= span)
                    & (levels[j] == y)) as u8;
            }
            let mut bits = 0;
            for k in 0..8 {
                let b = k << 3;
                let lanes = u64::from_le_bytes([
                    seen[b],
                    seen[b + 1],
                    seen[b + 2],
                    seen[b + 3],
                    seen[b + 4],
                    seen[b + 5],
                    seen[b + 6],
                    seen[b + 7],
                ]);
                bits |= (lanes.wrapping_mul(0x0102_0408_1020_4080) >> 56) << b;
            }
            out[w] = bits;
        }
    }

    /// The slots that moved or have an update block this tick.
    #[cfg(since_647)]
    pub fn changed(&self) -> &[u64; MAX_PLAYERS >> 6] {
        &self.changed
    }
}

impl Deref for PlayerSnapshots {
    type Target = [PlayerSnapshot];

    fn deref(&self) -> &[PlayerSnapshot] {
        &self.all[..]
    }
}

/// [`PlayerSnapshot`] for NPCs, indexed by nid.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NpcSnapshot {
    pub coord: u32,
    pub len: u16,
    pub code: u16,
    pub ntype: u16,
    pub bits: u8,
    pub flags: u8,
}

impl NpcSnapshot {
    pub const PRESENT: u8 = 1 << 0;
    pub const ACTIVE: u8 = 1 << 1;
    pub const TELE: u8 = 1 << 2;
    pub const JUMP: u8 = 1 << 3;

    pub const ABSENT: NpcSnapshot = NpcSnapshot {
        coord: 0,
        len: 0,
        code: 0,
        ntype: 0,
        bits: 0,
        flags: 0,
    };

    /// NPC `n` with an update block of `len` bytes.
    pub fn new(n: &Npc, len: u16) -> NpcSnapshot {
        let mut flags = NpcSnapshot::PRESENT;
        if n.active {
            flags |= NpcSnapshot::ACTIVE;
        }
        if n.pathing.tele {
            flags |= NpcSnapshot::TELE;
        }
        if n.pathing.jump {
            flags |= NpcSnapshot::JUMP;
        }
        let (code, bits) = Self::movement(&n.pathing);
        NpcSnapshot {
            coord: n.pathing.coord.packed(),
            len,
            code,
            ntype: n.uid.id(),
            bits,
            flags,
        }
    }

    #[cfg(before_647)]
    fn movement(pathing: &PathingEntity) -> (u16, u8) {
        steps(pathing.walk_dir, pathing.run_dir)
    }

    /// A run, a crawl (a run of one step), or a walk.
    #[cfg(since_647)]
    fn movement(pathing: &PathingEntity) -> (u16, u8) {
        if pathing.walk_dir == -1 {
            return (0, 0);
        }
        let walk = NPC_DIR_MAP[pathing.walk_dir as usize] as u16;
        if pathing.move_speed == MoveSpeed::Crawl {
            // 1 + 2 + 1 + 3 = 7 bits
            ((0b1100 << 3) | walk, 7)
        } else if pathing.run_dir != -1 {
            // 1 + 2 + 1 + 3 + 3 = 10 bits
            let run = NPC_DIR_MAP[pathing.run_dir as usize] as u16;
            ((0b1101 << 6) | (walk << 3) | run, 10)
        } else {
            // 1 + 2 + 3 = 6 bits
            ((0b101 << 3) | walk, 6)
        }
    }

    #[inline(always)]
    pub const fn clear(&mut self) {
        self.flags = 0;
    }

    /// Whether the tracked NPC has despawned, teleported, become inactive,
    /// or left the observer's level or view distance.
    #[inline(always)]
    pub fn should_remove(&self, obs_coord: CoordGrid, obs_y: u8, view_distance: u8) -> bool {
        self.flags & NpcSnapshot::PRESENT == 0
            || self.flags & NpcSnapshot::TELE != 0
            || CoordGrid::from(self.coord).y() != obs_y
            || !CoordGrid::in_distance(&obs_coord, CoordGrid::from(self.coord), view_distance)
            || self.flags & NpcSnapshot::ACTIVE == 0
    }
}

/// A run or a walk before 647, without the bit saying a block follows.
#[cfg(before_647)]
fn steps(walk_dir: i8, run_dir: i8) -> (u16, u8) {
    if run_dir != -1 {
        // 1 + 2 + 3 + 3 = 9 bits
        (
            (((1 << 2) | 2) << 6) | ((walk_dir as u16) << 3) | run_dir as u16,
            9,
        )
    } else if walk_dir != -1 {
        // 1 + 2 + 3 = 6 bits
        ((((1 << 2) | 1) << 3) | walk_dir as u16, 6)
    } else {
        (0, 0)
    }
}

/// A tracked entity's precomputed movement and whether its update block
/// follows, as one write; one that didn't move is 1 0 0 with a block, else 0.
#[inline(always)]
fn write_movement(bw: &mut BitWriter, buf: &mut Packet, code: u64, bits: u8, extend: bool) {
    let (code, width) = if bits > 0 {
        ((code << 1) | extend as u64, bits as u32 + 1)
    } else {
        ((extend as u64) << 2, 1 + ((extend as u32) << 1))
    };
    bw.pbits(buf, width, code);
}

#[cfg(since_647)]
const MAX_LOCAL_PLAYERS: usize = 255;

#[cfg(since_647)]
const WALK_DIR_MAP: [i32; 8] = [5, 6, 7, 3, 4, 0, 1, 2];
#[cfg(since_647)]
const NPC_DIR_MAP: [i32; 8] = [7, 0, 1, 6, 2, 5, 4, 3];

#[cfg(since_647)]
fn update_coordinates(bw: &mut BitWriter, buf: &mut Packet, last: u32, current: u32) {
    let last_plane = (last >> 16) & 0x3;
    let last_rx = (last >> 8) & 0xFF;
    let last_rz = last & 0xFF;
    let cur_plane = (current >> 16) & 0x3;
    let cur_rx = (current >> 8) & 0xFF;
    let cur_rz = current & 0xFF;
    let dp = cur_plane.wrapping_sub(last_plane) & 0x3;
    let dx = cur_rx as i32 - last_rx as i32;
    let dz = cur_rz as i32 - last_rz as i32;

    if last_rx == cur_rx && last_rz == cur_rz {
        bw.pbit::<2>(buf, 1);
        bw.pbit::<2>(buf, dp as i32);
    } else if dx.abs() <= 1 && dz.abs() <= 1 {
        let dir = WALK_DIR_MAP[face_dir(0, 0, dx, dz) as usize] as u32;
        bw.pbit::<2>(buf, 2);
        bw.pbit::<5>(buf, ((dp << 3) | dir) as i32);
    } else {
        bw.pbit::<2>(buf, 3);
        bw.pbit::<18>(
            buf,
            (((dz as u32) & 0xFF) + (((dx as u32) & 0xFF) << 8) + (dp << 16)) as i32,
        );
    }
}

/// A run of `count` skipped entries as one code, to go out with the entry
/// after it: the 0 flag, a 2-bit size class, then `count - 1` in 0, 5, 8 or
/// 11 bits. Nothing for an empty run. Without branches, as run lengths are
/// random.
#[cfg(since_647)]
const fn skip_code(count: usize) -> (u64, u32) {
    const WIDTHS: [u32; 4] = [3, 8, 11, 14];
    let n = (count as u64).wrapping_sub(1);
    let class = (n != 0) as usize + (n >= 32) as usize + (n >= 256) as usize;
    let width = WIDTHS[class];
    let some = ((count != 0) as u64).wrapping_neg();
    (
        (((class as u64) << (width - 3)) | n) & some,
        width & some as u32,
    )
}

/// How many slots in `from..to` are set.
#[cfg(since_647)]
fn slots(bits: &[u64], from: usize, to: usize) -> usize {
    let mut count = 0;
    let mut i = from;
    while i < to {
        let w = i >> 6;
        let end = (to - (w << 6)).min(64);
        let mask = (u64::MAX >> (64 - end)) & (u64::MAX << (i & 63));
        count += (bits[w] & mask).count_ones() as usize;
        i = (w + 1) << 6;
    }
    count
}

#[cfg(since_647)]
fn run_index(dx: i32, dz: i32) -> Option<i32> {
    Some(match (dx, dz) {
        (-2, -2) => 0,
        (-1, -2) => 1,
        (0, -2) => 2,
        (1, -2) => 3,
        (2, -2) => 4,
        (-2, -1) => 5,
        (2, -1) => 6,
        (-2, 0) => 7,
        (2, 0) => 8,
        (-2, 1) => 9,
        (2, 1) => 10,
        (-2, 2) => 11,
        (-1, 2) => 12,
        (0, 2) => 13,
        (1, 2) => 14,
        (2, 2) => 15,
        _ => return None,
    })
}

/// Encodes the player info update packet for a single game tick.
///
/// This struct owns the bit-packed output buffer and the byte-packed update
/// block buffer. Each tick, the engine calls [`encode`](PlayerInfo::encode)
/// to produce the complete player info payload that is sent to a single
/// observing player.
pub struct PlayerInfo {
    buf: Packet,
    updates: Packet,
    #[cfg(before_647)]
    tracked: Vec<u16>,
    bw: BitWriter,
    observer: Observer,
}

impl PlayerInfo {
    #[cfg(before_647)]
    const BITS_ADD: usize = 11 + 5 + 5 + 1 + 1;
    #[cfg(before_647)]
    const BITS_RUN: usize = 1 + 2 + 3 + 3 + 1;
    #[cfg(before_647)]
    const BITS_EXTEND: usize = 1 + 2;
    #[cfg(before_647)]
    const BITS_END: usize = 11;
    #[cfg(since_647)]
    const BITS_LOCAL: usize = 1 + 2 + 11 + 1 + 1 + 2 + 1 + 30;
    #[cfg(since_647)]
    const BITS_REGION: usize = 1 + 2 + 11 + 1 + 2 + 18;
    #[cfg(since_647)]
    const BITS_ADD: usize = 1 + 2 + 11 + 1 + 2 + 1 + 2 + 18 + 6 + 6 + 1;
    #[cfg(since_647)]
    const BITS_PASS: usize = 1 + 2 + 11 + 7;

    /// Creates a new `PlayerInfo` encoder with pre-allocated buffers.
    #[inline(always)]
    pub fn new() -> PlayerInfo {
        PlayerInfo {
            buf: Packet::new(MAX_PACKET_SIZE),
            updates: Packet::new(MAX_PACKET_SIZE),
            #[cfg(before_647)]
            tracked: Vec::with_capacity(BuildArea::PREFERRED_PLAYERS as usize),
            bw: BitWriter::new(),
            observer: Observer::default(),
        }
    }

    /// Encodes the full player info packet for a single tick.
    ///
    /// Rebuilds the build area if the observer has moved too far or a rebuild
    /// was requested, then writes the local player update, existing tracked
    /// players, and newly visible players into the bit-packed buffer. Update
    /// blocks are appended at the end.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn encode(
        &mut self,
        renderer: &mut PlayerRenderer,
        snap: &PlayerSnapshots,
        map: &ZoneMap,
        active: &mut ActivePlayer,
        dx: i32,
        dz: i32,
        rebuild: bool,
    ) -> (&[u8], &[u8]) {
        self.observer = Observer {
            pid: active.player.uid.pid(),
            #[cfg(before_647)]
            origin: active.player.build_area.origin,
        };
        #[cfg(before_647)]
        {
            let (px, py, pz) = (
                active.player.pathing.coord.x(),
                active.player.pathing.coord.y(),
                active.player.pathing.coord.z(),
            );
            let self_pid = active.player.uid.pid();
            let build: &mut BuildArea = &mut active.player.build_area;

            if rebuild || dx > build.view_distance as i32 || dz > build.view_distance as i32 {
                if active.watch.checking() {
                    let clock = engine().clock;
                    for &pid in build.players.iter() {
                        active.watch.anticheat.lost(Target::Player(pid), clock);
                    }
                }
                build.rebuild_players(snap, map, px, py, pz, self_pid);
            } else {
                build.resize();
            }
        }
        #[cfg(since_647)]
        {
            let _ = (dx, dz, rebuild, map);
            active.player.build_area.resize();
        }

        self.updates.pos = 0;
        self.bw.reset();

        #[cfg(before_647)]
        {
            self.write_local_player(renderer, snap, active);
            self.write_players(snap, renderer, active);
            self.write_new_players(map, snap, renderer, active);
            if self.updates.pos > 0 {
                self.bw
                    .pbit::<{ Self::BITS_END }>(&mut self.buf, MAX_PLAYERS as i32 - 1);
            }
        }
        #[cfg(since_647)]
        self.write_players(snap, renderer, active);
        self.bw.finish(&mut self.buf);
        debug_assert!(self.buf.pos + self.updates.pos <= BYTES_LIMIT);
        unsafe {
            (
                self.buf.data.get_unchecked(..self.buf.pos),
                self.updates.data.get_unchecked(..self.updates.pos),
            )
        }
    }

    /// The observer's own movement, as a teleport within the build area or
    /// its steps, and its update block.
    #[inline(always)]
    #[cfg(before_647)]
    fn write_local_player(
        &mut self,
        renderer: &mut PlayerRenderer,
        snap: &[PlayerSnapshot],
        active: &ActivePlayer,
    ) {
        let pid = active.player.uid.pid();
        let extend = renderer.highdefinitions(pid) > 0;
        let pathing = &active.player.pathing;
        if pathing.tele {
            let origin = active.player.build_area.origin;
            self.teleport(
                pathing.coord.x() as i32 - origin.zone_origin_x() as i32,
                pathing.coord.y() as i32,
                pathing.coord.z() as i32 - origin.zone_origin_z() as i32,
                pathing.jump,
                extend,
            );
        } else {
            let s = unsafe { *snap.get_unchecked(pid as usize) };
            write_movement(&mut self.bw, &mut self.buf, s.code, s.bits, extend);
        }
        if extend {
            renderer.write_high_block_for(&mut self.updates, pid, self.observer);
        }
    }

    /// The tracked players' movement, read from the snapshot, with each update
    /// block that still fits ahead of the bits every later entry must have.
    /// Players gone from view are removed.
    #[inline(always)]
    #[cfg(before_647)]
    fn write_players(
        &mut self,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        active: &mut ActivePlayer,
    ) {
        active.player.build_area.players.swap_ids(&mut self.tracked);
        self.bw.pbit::<8>(&mut self.buf, self.tracked.len() as i32);

        let obs_coord = active.player.pathing.coord;
        let obs_y = obs_coord.y();
        let view_distance = active.player.build_area.view_distance;

        for i in 0..self.tracked.len() {
            let pid = self.tracked[i];
            let s = unsafe { *snap.get_unchecked(pid as usize) };
            if s.should_remove(obs_coord, obs_y, view_distance) {
                self.remove(active, pid);
                continue;
            }
            let later = (self.tracked.len() - i - 1) * Self::BITS_RUN + Self::BITS_END;
            let bits = if s.bits > 0 {
                s.bits as usize + 1
            } else {
                Self::BITS_EXTEND
            };
            let extend = s.len > 0 && self.fits(bits + later, s.len as usize);
            write_movement(&mut self.bw, &mut self.buf, s.code, s.bits, extend);
            if extend {
                renderer.write_high_block_for(&mut self.updates, pid, self.observer);
            }
        }
        active.player.build_area.players.swap_ids(&mut self.tracked);
        active.player.build_area.players.retain_bits();
    }

    /// The four passes 647 reads: tracked players that were active last tick,
    /// then the ones skipped, then untracked players skipped last tick, then
    /// the rest. Which slots the observer sees is worked out first, as bits,
    /// so each pass only stops at the players that left view, moved, have a
    /// block, can be added or had their region moved, and the skips between
    /// them are counted from the slot bits. `later` holds back the bits every
    /// entry still to come must have, so the blocks and adds that don't fit
    /// wait for a later tick.
    #[cfg(since_647)]
    fn write_players(
        &mut self,
        snap: &PlayerSnapshots,
        renderer: &mut PlayerRenderer,
        active: &mut ActivePlayer,
    ) {
        let build = &active.player.build_area;
        let mut view = [0; MAX_PLAYERS >> 6];
        snap.view(active.player.pathing.coord, build.view_distance, &mut view);
        let mut additions = build.locals.iter().map(|w| w.count_ones() as usize).sum();
        let moves: usize = renderer
            .region_moves()
            .iter()
            .map(|bits| bits.count_ones() as usize)
            .sum();
        let mut later =
            additions * Self::BITS_LOCAL + moves * Self::BITS_REGION + 4 * Self::BITS_PASS;
        for skipped in [false, true] {
            self.write_local_players(snap, &view, renderer, active, skipped, &mut later);
            self.bw.finish(&mut self.buf);
        }
        for skipped in [true, false] {
            self.write_external_players(
                snap,
                &view,
                renderer,
                active,
                skipped,
                &mut additions,
                &mut later,
            );
            self.bw.finish(&mut self.buf);
        }
        active.player.build_area.cycle_nsn_flags();
    }

    /// Writes add-entries for newly visible players that are not yet tracked.
    ///
    /// Queries the build area for nearby players, skips those already tracked
    /// or hard-hidden, and encodes an add-entry for each until the buffer
    /// limit or the preferred player cap is reached.
    #[inline(always)]
    #[cfg(before_647)]
    fn write_new_players(
        &mut self,
        map: &ZoneMap,
        snap: &PlayerSnapshots,
        renderer: &mut PlayerRenderer,
        active: &mut ActivePlayer,
    ) {
        let (px, py, pz) = (
            active.player.pathing.coord.x(),
            active.player.pathing.coord.y(),
            active.player.pathing.coord.z(),
        );
        let self_pid = active.player.uid.pid();
        active
            .player
            .build_area
            .get_nearby_players(snap, map, px, py, pz, self_pid);
        let mut i = 0;
        while i < active.player.build_area.nearby_players.len() {
            let pid = active.player.build_area.nearby_players[i];
            i += 1;
            if active.player.build_area.players.contains(pid) {
                continue;
            }
            if active.player.build_area.players.len() >= BuildArea::PREFERRED_PLAYERS as usize {
                return;
            }
            let s = unsafe { *snap.get_unchecked(pid as usize) };
            if s.flags & (PlayerSnapshot::PRESENT | PlayerSnapshot::VIS_HARD)
                != PlayerSnapshot::PRESENT
            {
                continue;
            }
            let len: usize = renderer.lowdefinitions(pid) + renderer.highdefinitions(pid);
            if !self.fits(PlayerInfo::BITS_ADD + PlayerInfo::BITS_END, len) {
                return;
            }
            let coord = CoordGrid::from(s.coord);
            self.add(
                renderer,
                active,
                pid,
                coord.x() as i32 - active.player.pathing.coord.x() as i32,
                coord.z() as i32 - active.player.pathing.coord.z() as i32,
                s.flags & PlayerSnapshot::JUMP != 0,
            );
        }
    }

    /// Encodes a player-add entry (23 bits) with position delta and jump flag,
    /// followed by the low-definition update block.
    #[inline(always)]
    #[cfg(before_647)]
    fn add(
        &mut self,
        renderer: &mut PlayerRenderer,
        active: &mut ActivePlayer,
        pid: u16,
        x: i32,
        z: i32,
        jump: bool,
    ) {
        // 11 + 5 + 5 + 1 + 1 = 23 bits
        self.bw.pbit::<23>(
            &mut self.buf,
            ((pid as i32) << 12) | ((x & 0x1F) << 7) | ((z & 0x1F) << 2) | ((jump as i32) << 1) | 1,
        );
        renderer.ready_low(pid);
        let appearance = Self::low_appearance(renderer, active, pid);
        renderer.write_low_block_for(&mut self.updates, pid, self.observer, appearance);
        active.player.build_area.players.insert(pid);
    }

    /// Encodes a player-remove entry (3 bits) and clears the player's bit
    /// in the observer's tracked set.
    ///
    /// Only clears the bit (O(1)); the caller reconciles the ID list after
    /// the loop via [`IdBitSet::retain_bits`].
    #[inline(always)]
    #[cfg(before_647)]
    fn remove(&mut self, active: &mut ActivePlayer, other: u16) {
        // 1 + 2 = 3 bits
        self.bw.pbit::<3>(&mut self.buf, (1 << 2) | 3);
        active.player.build_area.players.remove_bit(other);
        if active.watch.checking() {
            active
                .watch
                .anticheat
                .lost(Target::Player(other), engine().clock);
        }
    }

    /// A teleport to `x`, `z` within the build area, on level `y`.
    #[inline(always)]
    #[cfg(before_647)]
    fn teleport(&mut self, x: i32, y: i32, z: i32, jump: bool, extend: bool) {
        // 1 + 2 + 2 + 7 + 7 + 1 + 1 = 21 bits
        self.bw.pbit::<21>(
            &mut self.buf,
            (((1 << 2) | 3) << 18)
                | ((y & 0x3) << 16)
                | ((x & 0x7F) << 9)
                | ((z & 0x7F) << 2)
                | ((jump as i32) << 1)
                | (extend as i32),
        );
    }

    #[inline(always)]
    #[cfg(before_647)]
    fn idle(&mut self) {
        self.bw.pbit::<1>(&mut self.buf, 0);
    }

    /// Whether a newly added player's blocks carry its appearance: only when
    /// the observer doesn't already have it.
    #[inline(always)]
    fn low_appearance(renderer: &PlayerRenderer, active: &mut ActivePlayer, pid: u16) -> bool {
        match renderer.appearance_clock(pid) {
            Some(clock) if !active.player.build_area.has_appearance(pid, clock) => {
                active.player.build_area.save_appearance(pid, clock);
                true
            }
            _ => false,
        }
    }

    /// Checks whether the buffer has room for the given number of additional
    /// bits and bytes without exceeding the packet size limit.
    #[inline(always)]
    fn fits(&self, bits_to_add: usize, bytes_to_add: usize) -> bool {
        room(&self.bw, &self.updates, bits_to_add, bytes_to_add)
    }

    /// A session replay's keyframe (see [`crate::watch`]): `PLAYER_INFO`
    /// bodies that bring a client that has just logged in to what `active`'s
    /// client shows now. The local player stands where they are, and the
    /// players its client tracks are added in the same order, so the next
    /// live update applies. More than one body when it all doesn't fit in
    /// one. Only the players in view are handed over: the caller has the
    /// server forget the others' appearances (see
    /// [`BuildArea::forget_untracked_appearances`]).
    ///
    /// Reads the observer's state without changing it.
    #[cfg(before_647)]
    pub fn keyframe(
        renderer: &mut PlayerRenderer,
        players: OtherPlayers<'_>,
        active: &ActivePlayer,
    ) -> Vec<Vec<u8>> {
        let build = &active.player.build_area;
        let tracked = build.players.iter();
        let mut info = PlayerInfo::new();
        let mut known = 0usize;
        let mut bodies = Vec::new();
        let coord = active.player.pathing.coord;
        loop {
            info.updates.pos = 0;
            info.bw.reset();
            if bodies.is_empty() {
                info.teleport(
                    coord.x() as i32 - build.origin.zone_origin_x() as i32,
                    coord.y() as i32,
                    coord.z() as i32 - build.origin.zone_origin_z() as i32,
                    true,
                    true,
                );
                renderer.write_keyframe_block(
                    &mut info.updates,
                    active.player.uid.pid(),
                    &active.player.info,
                    coord,
                );
            } else {
                info.idle();
            }
            info.bw.pbit::<8>(&mut info.buf, known as i32);
            for _ in 0..known {
                info.idle();
            }
            while known < tracked.len() {
                let pid = tracked[known];
                let other = players.get(pid);
                let len = other.map_or(1, |_| renderer.lowdefinitions(pid) + 32);
                if !info.fits(PlayerInfo::BITS_ADD + PlayerInfo::BITS_END, len) {
                    break;
                }
                let (x, z) = other.map_or((0, 0), |other| {
                    (
                        other.player.pathing.coord.x() as i32 - coord.x() as i32,
                        other.player.pathing.coord.z() as i32 - coord.z() as i32,
                    )
                });
                // 11 + 5 + 5 + 1 + 1 = 23 bits
                info.bw.pbit::<23>(
                    &mut info.buf,
                    ((pid as i32) << 12) | ((x & 0x1F) << 7) | ((z & 0x1F) << 2) | (1 << 1) | 1,
                );
                match other {
                    Some(other) => renderer.write_keyframe_block(
                        &mut info.updates,
                        pid,
                        &other.player.info,
                        other.player.pathing.coord,
                    ),
                    // Gone this tick: the live stream drops it next.
                    None => info.updates.p1(0),
                }
                known += 1;
            }
            if info.updates.pos > 0 {
                info.bw
                    .pbit::<{ PlayerInfo::BITS_END }>(&mut info.buf, MAX_PLAYERS as i32 - 1);
                info.bw.finish(&mut info.buf);
                info.buf.pdata(&info.updates.data, 0, info.updates.pos);
            } else {
                info.bw.finish(&mut info.buf);
            }
            bodies.push(info.buf.data[..info.buf.pos].to_vec());
            if known >= tracked.len() {
                return bodies;
            }
        }
    }

    #[cfg(since_647)]
    fn write_local_players(
        &mut self,
        snap: &PlayerSnapshots,
        view: &[u64; MAX_PLAYERS >> 6],
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        skipped: bool,
        later: &mut usize,
    ) {
        let self_pid = observer.player.uid.pid() as usize;
        let build = &observer.player.build_area;
        let mut pass = [0u64; MAX_PLAYERS >> 6];
        for (w, bits) in pass.iter_mut().enumerate() {
            let flags = if skipped {
                build.skipped[w]
            } else {
                !build.skipped[w]
            };
            *bits = build.locals[w] & flags;
        }
        let mut written = [0u64; MAX_PLAYERS >> 6];
        let (mut before, mut run, mut counted) = (0, 0, 0);
        for w in 0..pass.len() {
            let mut gone = pass[w] & !view[w];
            if w == self_pid >> 6 {
                gone &= !(1 << (self_pid & 63));
            }
            let mut bits = gone | (pass[w] & snap.changed()[w]);
            while bits != 0 {
                let bit = bits & bits.wrapping_neg();
                bits ^= bit;
                let index = (w << 6) | bit.trailing_zeros() as usize;
                let at = before + (pass[w] & (bit - 1)).count_ones() as usize;
                *later -= Self::BITS_LOCAL * (at + 1 - counted);
                counted = at + 1;
                let removing = gone & bit != 0;
                let s = unsafe { *snap.get_unchecked(index) };
                let len = renderer.highdefinitions(index as u16);
                let updating = !removing & (len > 0) & self.fits(Self::BITS_LOCAL + *later, len);
                if !(removing || s.bits != 0 || updating) {
                    continue;
                }
                written[w] |= bit;
                let (code, count) = skip_code(at - run);
                run = at + 1;
                let b = s.bits.max(2) as u32;
                let (entry, width) = if removing {
                    (0b1000, 4)
                } else {
                    ((1 << (b + 1)) | s.code | (updating as u64) << b, b + 2)
                };
                self.bw
                    .pbits(&mut self.buf, count + width, (code << width) | entry);
                if removing {
                    self.remove_player(renderer, observer, index);
                } else if updating {
                    renderer.write_high_block_for(&mut self.updates, index as u16, self.observer);
                }
            }
            before += pass[w].count_ones() as usize;
        }
        *later -= Self::BITS_LOCAL * (before - counted);
        let (code, count) = skip_code(before - run);
        self.bw.pbits(&mut self.buf, count, code);
        let skipping = &mut observer.player.build_area.skipping;
        for w in 0..MAX_PLAYERS >> 6 {
            skipping[w] |= pass[w] & !written[w];
        }
    }

    /// The client goes back to keeping the player's region, taking it from
    /// the tile it last had them on: last tick's region, which moves with
    /// every other client's.
    #[cfg(since_647)]
    fn remove_player(
        &mut self,
        renderer: &PlayerRenderer,
        observer: &mut ActivePlayer,
        index: usize,
    ) {
        self.write_region(renderer, index as u16);
        observer.player.build_area.players.remove(index as u16);
        if observer.watch.checking() {
            observer
                .watch
                .anticheat
                .lost(Target::Player(index as u16), engine().clock);
        }
    }

    #[cfg(since_647)]
    #[allow(clippy::too_many_arguments)]
    fn write_external_players(
        &mut self,
        snap: &PlayerSnapshots,
        view: &[u64; MAX_PLAYERS >> 6],
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        skipped: bool,
        additions: &mut usize,
        later: &mut usize,
    ) {
        let build = &observer.player.build_area;
        let mut pass = [0u64; MAX_PLAYERS >> 6];
        for (w, bits) in pass.iter_mut().enumerate() {
            let flags = if skipped {
                build.skipped[w]
            } else {
                !build.skipped[w]
            };
            *bits = !build.locals[w] & flags;
        }
        pass[0] &= !1;
        let mut moved = [0u64; MAX_PLAYERS >> 6];
        let mut from = 0;
        for w in 0..pass.len() {
            let moves = renderer.region_moves()[w];
            let open = if *additions < MAX_LOCAL_PLAYERS {
                view[w]
            } else {
                0
            };
            let mut bits = (open | moves) & pass[w];
            while bits != 0 {
                let bit = bits & bits.wrapping_neg();
                bits ^= bit;
                let index = (w << 6) | bit.trailing_zeros() as usize;
                if moves & bit != 0 {
                    *later -= Self::BITS_REGION;
                }
                let adding = open & bit != 0
                    && *additions < MAX_LOCAL_PLAYERS
                    && self.fits(
                        Self::BITS_ADD + *later,
                        renderer.lowdefinitions(index as u16)
                            + renderer.highdefinitions(index as u16),
                    );
                if !adding && moves & bit == 0 {
                    continue;
                }
                let (code, count) = skip_code(slots(&pass, from, index));
                self.bw.pbits(&mut self.buf, count + 1, (code << 1) | 1);
                if adding {
                    self.add_player(snap, renderer, observer, index);
                    *additions += 1;
                    if *additions == MAX_LOCAL_PLAYERS {
                        bits &= moves;
                    }
                } else {
                    update_coordinates(
                        &mut self.bw,
                        &mut self.buf,
                        renderer.last_region(index as u16),
                        renderer.region(index as u16),
                    );
                    moved[w] |= bit;
                }
                from = index + 1;
            }
        }
        let (code, count) = skip_code(slots(&pass, from, MAX_PLAYERS));
        self.bw.pbits(&mut self.buf, count, code);
        let skipping = &mut observer.player.build_area.skipping;
        for w in 0..MAX_PLAYERS >> 6 {
            skipping[w] |= pass[w] & !moved[w];
        }
    }

    #[cfg(since_647)]
    fn add_player(
        &mut self,
        snap: &PlayerSnapshots,
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        index: usize,
    ) {
        let coord = CoordGrid::from(unsafe { *snap.get_unchecked(index) }.coord);

        self.bw.pbit::<2>(&mut self.buf, 0);
        self.write_region(renderer, index as u16);
        self.bw.pbit::<6>(&mut self.buf, (coord.x() & 0x3F) as i32);
        self.bw.pbit::<6>(&mut self.buf, (coord.z() & 0x3F) as i32);
        self.bw.pbit::<1>(&mut self.buf, 1);

        renderer.ready_low(index as u16);
        let appearance = Self::low_appearance(renderer, observer, index as u16);
        renderer.write_low_block_for(&mut self.updates, index as u16, self.observer, appearance);

        observer.player.build_area.players.insert(index as u16);
    }

    /// Moves the region a client keeps for `pid` from last tick's to this
    /// tick's, if it moved.
    #[cfg(since_647)]
    fn write_region(&mut self, renderer: &PlayerRenderer, pid: u16) {
        let (last, region) = (renderer.last_region(pid), renderer.region(pid));
        self.bw.pbit::<1>(&mut self.buf, (region != last) as i32);
        if region != last {
            update_coordinates(&mut self.bw, &mut self.buf, last, region);
        }
    }
}

/// Encodes the NPC info update packet for a single game tick.
///
/// Analogous to [`PlayerInfo`] but for NPC entities. Each tick, the engine
/// calls [`encode`](NpcInfo::encode) to produce the NPC info payload sent
/// to a single observing player.
pub struct NpcInfo {
    buf: Packet,
    updates: Packet,
    tracked: Vec<u16>,
    bw: BitWriter,
    observer: u16,
}

impl NpcInfo {
    #[cfg(before_254)]
    const BITS_ADD: usize = 13 + 11 + 5 + 5 + 1;
    #[cfg(all(since_254, before_274))]
    const BITS_ADD: usize = 14 + 11 + 5 + 5 + 1;
    #[cfg(all(since_274, before_647))]
    const BITS_ADD: usize = 14 + 11 + 5 + 5 + 1 + 1;
    #[cfg(before_647)]
    const BITS_RUN: usize = 1 + 2 + 3 + 3 + 1;
    #[cfg(since_647)]
    const BITS_RUN: usize = 1 + 2 + 1 + 3 + 3 + 1;
    #[cfg(since_647)]
    const BITS_ADD: usize = 15 + 2 + 5 + 1 + 3 + 5 + 14 + 1;
    const BITS_EXTEND: usize = 1 + 2;
    #[cfg(before_254)]
    const BITS_END: usize = 13;
    #[cfg(all(since_254, before_647))]
    const BITS_END: usize = 14;
    #[cfg(since_647)]
    const BITS_END: usize = 15;

    /// Creates a new `NpcInfo` encoder with pre-allocated buffers.
    #[inline(always)]
    pub fn new() -> NpcInfo {
        NpcInfo {
            buf: Packet::new(MAX_PACKET_SIZE),
            updates: Packet::new(MAX_PACKET_SIZE),
            tracked: Vec::with_capacity(BuildArea::PREFERRED_NPCS as usize),
            bw: BitWriter::new(),
            observer: 0,
        }
    }

    /// Encodes the full NPC info packet for a single tick.
    ///
    /// Rebuilds the NPC build area if the observer moved too far or a rebuild
    /// was forced, then writes existing tracked NPCs and newly visible NPCs
    /// into the bit-packed buffer. Update blocks are appended at the end.
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn encode(
        &mut self,
        renderer: &mut NpcRenderer,
        npcs: &mut [Option<Box<ActiveNpc>>],
        nsnap: &[NpcSnapshot],
        map: &ZoneMap,
        active: &mut ActivePlayer,
        dx: i32,
        dz: i32,
        rebuild: bool,
    ) -> (&[u8], &[u8]) {
        self.observer = active.player.uid.pid();
        let build: &mut BuildArea = &mut active.player.build_area;

        if rebuild
            || dx > BuildArea::PREFERRED_VIEW_DISTANCE as i32
            || dz > BuildArea::PREFERRED_VIEW_DISTANCE as i32
        {
            if active.watch.checking() {
                let clock = engine().clock;
                for &nid in build.npcs.iter() {
                    active.watch.anticheat.lost(Target::Npc(nid), clock);
                }
            }
            build.rebuild_npcs(npcs);
        }

        self.updates.pos = 0;
        self.bw.reset();

        self.write_npcs(npcs, nsnap, renderer, active);
        self.write_new_npcs(map, npcs, nsnap, renderer, active);
        if cfg!(since_647) || self.updates.pos > 0 {
            self.bw
                .pbit::<{ Self::BITS_END }>(&mut self.buf, MAX_NPCS as i32 - 1);
        }
        self.bw.finish(&mut self.buf);
        debug_assert!(self.buf.pos + self.updates.pos <= BYTES_LIMIT);
        unsafe {
            (
                self.buf.data.get_unchecked(..self.buf.pos),
                self.updates.data.get_unchecked(..self.updates.pos),
            )
        }
    }

    /// Writes movement and update blocks for all previously tracked NPCs.
    ///
    /// Iterates over the current tracked NPC set. NPCs that have teleported,
    /// changed level, moved out of view, or gone inactive are removed
    /// (decrementing their observer count). Remaining NPCs are encoded with
    /// their current movement mode.
    #[inline(always)]
    fn write_npcs(
        &mut self,
        npcs: &mut [Option<Box<ActiveNpc>>],
        nsnap: &[NpcSnapshot],
        renderer: &mut NpcRenderer,
        active: &mut ActivePlayer,
    ) {
        active.player.build_area.npcs.swap_ids(&mut self.tracked);
        let mut bw = mem::replace(&mut self.bw, BitWriter::new());
        bw.pbit::<8>(&mut self.buf, self.tracked.len() as i32);

        let obs_coord = active.player.pathing.coord;
        let obs_y = obs_coord.y();

        for i in 0..self.tracked.len() {
            let nid = self.tracked[i];
            let s = unsafe { *nsnap.get_unchecked(nid as usize) };
            if s.should_remove(obs_coord, obs_y, BuildArea::PREFERRED_VIEW_DISTANCE) {
                if s.flags & NpcSnapshot::PRESENT != 0 {
                    unsafe {
                        if let Some(other) = (*npcs.as_mut_ptr().add(nid as usize)).as_mut() {
                            other.npc.observers = other.npc.observers.saturating_sub(1);
                        }
                    }
                }
                Self::remove(&mut bw, &mut self.buf, active, nid);
                continue;
            }

            let later = (self.tracked.len() - i - 1) * NpcInfo::BITS_RUN + NpcInfo::BITS_END;
            let bits = if s.bits > 0 {
                s.bits as usize + 1
            } else {
                Self::BITS_EXTEND
            };
            let extend = s.len > 0 && room(&bw, &self.updates, bits + later, s.len as usize);
            write_movement(&mut bw, &mut self.buf, s.code as u64, s.bits, extend);
            if extend {
                renderer.write_high_block_for(&mut self.updates, nid, self.observer);
            }
        }
        self.bw = bw;
        active.player.build_area.npcs.swap_ids(&mut self.tracked);
        active.player.build_area.npcs.retain_bits();
    }

    /// Writes add-entries for newly visible NPCs that are not yet tracked.
    ///
    /// Queries the build area for nearby NPCs, skips those already tracked,
    /// and encodes an add-entry for each until the buffer limit or the
    /// preferred NPC cap is reached. Increments the observer count on each
    /// newly tracked NPC.
    #[inline(always)]
    fn write_new_npcs(
        &mut self,
        map: &ZoneMap,
        npcs: &mut [Option<Box<ActiveNpc>>],
        nsnap: &[NpcSnapshot],
        renderer: &mut NpcRenderer,
        active: &mut ActivePlayer,
    ) {
        let (px, py, pz) = (
            active.player.pathing.coord.x(),
            active.player.pathing.coord.y(),
            active.player.pathing.coord.z(),
        );
        active
            .player
            .build_area
            .get_nearby_npcs(nsnap, map, px, py, pz);
        let mut i = 0;
        while i < active.player.build_area.nearby_npcs.len() {
            let nid = active.player.build_area.nearby_npcs[i];
            i += 1;
            if active.player.build_area.npcs.contains(nid) {
                continue;
            }
            if active.player.build_area.npcs.len() >= BuildArea::PREFERRED_NPCS as usize {
                return;
            }
            if let Some(other) = unsafe { &mut *npcs.as_mut_ptr().add(nid as usize) } {
                let len = NpcRenderer::LOW_LEN + renderer.highdefinitions(nid);
                if !self.fits(Self::BITS_ADD + Self::BITS_END, len) {
                    return;
                }
                self.add(renderer, active, nid, unsafe {
                    *nsnap.get_unchecked(nid as usize)
                });
                other.npc.observers = other.npc.observers.saturating_add(1);
            }
        }
    }

    /// Encodes an NPC-add entry (35 bits split across two pbit calls) with
    /// the NPC ID, type, position delta, and low-definition update block,
    /// all from the snapshot and what the renderer noted.
    #[inline(always)]
    fn add(
        &mut self,
        renderer: &mut NpcRenderer,
        active: &mut ActivePlayer,
        nid: u16,
        s: NpcSnapshot,
    ) {
        let coord = CoordGrid::from(s.coord);
        let x = coord.x() as i32 - active.player.pathing.coord.x() as i32;
        let z = coord.z() as i32 - active.player.pathing.coord.z() as i32;
        let ntype = s.ntype;
        // 13/14 + 11 = 24/25 bits, then 5 + 5 + (1) + 1 = 11/12 bits (35/36/37 total, split for i32)
        #[cfg(before_254)]
        self.bw
            .pbit::<24>(&mut self.buf, ((nid as i32) << 11) | (ntype as i32));
        #[cfg(all(since_254, before_647))]
        self.bw
            .pbit::<25>(&mut self.buf, ((nid as i32) << 11) | (ntype as i32));
        #[cfg(before_274)]
        self.bw
            .pbit::<11>(&mut self.buf, ((x & 0x1F) << 6) | ((z & 0x1F) << 1) | 1);
        #[cfg(all(since_274, before_647))]
        {
            let jump = s.flags & NpcSnapshot::JUMP != 0;
            self.bw.pbit::<12>(
                &mut self.buf,
                ((x & 0x1F) << 7) | ((z & 0x1F) << 2) | ((jump as i32) << 1) | 1,
            );
        }
        #[cfg(since_647)]
        {
            self.bw.pbit::<15>(&mut self.buf, nid as i32);
            self.bw.pbit::<2>(&mut self.buf, coord.y() as i32);
            self.bw.pbit::<5>(&mut self.buf, x);
            self.bw.pbit::<1>(&mut self.buf, 1);
            self.bw
                .pbit::<3>(&mut self.buf, renderer.add_direction(nid) as i32);
            self.bw.pbit::<5>(&mut self.buf, z);
            self.bw.pbit::<14>(&mut self.buf, ntype as i32);
            self.bw.pbit::<1>(&mut self.buf, 0);
        }
        renderer.ready_low(nid);
        renderer.write_low_block_for(&mut self.updates, nid, self.observer);
        active.player.build_area.npcs.insert(nid);
    }

    /// Encodes an NPC-remove entry (3 bits) and clears the NPC's bit in
    /// the observer's tracked set.
    ///
    /// Only clears the bit (O(1)); the caller reconciles the ID list after
    /// the loop via [`IdBitSet::retain_bits`].
    #[inline(always)]
    fn remove(bw: &mut BitWriter, buf: &mut Packet, active: &mut ActivePlayer, other: u16) {
        // 1 + 2 = 3 bits
        bw.pbit::<3>(buf, (1 << 2) | 3);
        active.player.build_area.npcs.remove_bit(other);
        if active.watch.checking() {
            active
                .watch
                .anticheat
                .lost(Target::Npc(other), engine().clock);
        }
    }

    /// Checks whether the buffer has room for the given number of additional
    /// bits and bytes without exceeding the NPC info packet size limit.
    #[inline(always)]
    fn fits(&self, bits_to_add: usize, bytes_to_add: usize) -> bool {
        room(&self.bw, &self.updates, bits_to_add, bytes_to_add)
    }

    /// A session replay's keyframe (see [`PlayerInfo::keyframe`]): `NPC_INFO`
    /// bodies that add the NPCs `active`'s client tracks, in the same order.
    /// More than one when they don't all fit in one.
    ///
    /// Reads the observer's state without changing it.
    #[cfg(before_647)]
    pub fn keyframe(
        renderer: &mut NpcRenderer,
        npcs: &[Option<Box<ActiveNpc>>],
        active: &ActivePlayer,
    ) -> Vec<Vec<u8>> {
        let tracked = active.player.build_area.npcs.iter();
        let coord = active.player.pathing.coord;
        let mut info = NpcInfo::new();
        let mut known = 0usize;
        let mut bodies = Vec::new();
        loop {
            info.updates.pos = 0;
            info.bw.reset();
            info.bw.pbit::<8>(&mut info.buf, known as i32);
            for _ in 0..known {
                info.bw.pbit::<1>(&mut info.buf, 0);
            }
            let first = known;
            while known < tracked.len() {
                let nid = tracked[known];
                let other = npcs.get(nid as usize).and_then(Option::as_ref);
                let len = other.map_or(1, |_| NpcRenderer::LOW_LEN + 16);
                if !info.fits(NpcInfo::BITS_ADD + NpcInfo::BITS_END, len) {
                    break;
                }
                let (ntype, x, z) = other.map_or((0, 0, 0), |n| {
                    (
                        n.npc.uid.id() as i32,
                        n.npc.pathing.coord.x() as i32 - coord.x() as i32,
                        n.npc.pathing.coord.z() as i32 - coord.z() as i32,
                    )
                });
                #[cfg(before_254)]
                info.bw
                    .pbit::<24>(&mut info.buf, ((nid as i32) << 11) | ntype);
                #[cfg(since_254)]
                info.bw
                    .pbit::<25>(&mut info.buf, ((nid as i32) << 11) | ntype);
                #[cfg(before_274)]
                info.bw
                    .pbit::<11>(&mut info.buf, ((x & 0x1F) << 6) | ((z & 0x1F) << 1) | 1);
                #[cfg(since_274)]
                info.bw
                    .pbit::<12>(&mut info.buf, ((x & 0x1F) << 7) | ((z & 0x1F) << 2) | 2 | 1);
                match other {
                    Some(other) => renderer.write_keyframe_block(
                        &mut info.updates,
                        nid,
                        &other.npc.info,
                        other.npc.pathing.coord,
                    ),
                    // Gone this tick: the live stream drops it next.
                    None => info.updates.p1(0),
                }
                known += 1;
            }
            if info.updates.pos > 0 {
                info.bw
                    .pbit::<{ NpcInfo::BITS_END }>(&mut info.buf, MAX_NPCS as i32 - 1);
                info.bw.finish(&mut info.buf);
                info.buf.pdata(&info.updates.data, 0, info.updates.pos);
            } else {
                info.bw.finish(&mut info.buf);
            }
            bodies.push(info.buf.data[..info.buf.pos].to_vec());
            // A body that couldn't fit even one more stops here rather than
            // loop forever.
            if known >= tracked.len() || known == first {
                return bodies;
            }
        }
    }
}

#[cfg(all(test, since_647))]
mod client_decode_tests {
    use super::*;
    use crate::clients::client_game::create_io;
    use rs_crypto::isaac::IsaacPair;
    use rs_entity::{Player, PlayerUid};
    use rs_info::EntityMasks;
    use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};
    use rs_var::VarSet;

    fn observer() -> ActivePlayer {
        let coord = CoordGrid::new(3222, 0, 3218);
        let mut player = Player::new(
            PlayerUid::new("observer".into(), 1),
            coord,
            VarSet::new([].into_iter()),
            false,
        );
        player.build_area.init_nsn_state(1);
        ActivePlayer {
            player,
            handle: Box::new(create_io(IsaacPair::new(&[0; 4], &[0; 4])).handle),
            buffered: Packet::new(0),
            buffered_starts: Vec::new(),
            client_limit: 0,
            user_limit: 0,
            restricted_limit: 0,
            remote_ip: None,
            muted_until: None,
            unread_messages: 0,
            last_report: None,
            last_social: None,
            friends: Vec::new(),
            ignores: Vec::new(),
            watch: Box::default(),
            capture: None,
            pending_path: None,
        }
    }

    fn bits(bytes: &[u8], pos: &mut usize, count: usize) -> u32 {
        let mut value = 0;
        for _ in 0..count {
            value = (value << 1) | u32::from(bytes[*pos >> 3] >> (7 - (*pos & 7)) & 1);
            *pos += 1;
        }
        value
    }

    #[test]
    fn npc_movement_survives_a_full_update_buffer() {
        let mut active = observer();
        let mut renderer = NpcRenderer::new();
        let mut info = NpcInfo::new();
        let mut snap = vec![NpcSnapshot::ABSENT; MAX_NPCS];
        let mut npcs = (0..MAX_NPCS).map(|_| None).collect::<Vec<_>>();
        let mut masks = EntityMasks::new();
        masks.masks = NpcInfoProt::Say as u16;
        masks.say = Some("a".repeat(1000).into());
        for nid in 1..=250 {
            active.player.build_area.npcs.insert(nid);
            renderer.compute_info(nid as usize, &masks);
            let coord = active.player.pathing.coord;
            let mut npc = Npc::new(0, nid, coord, 1, VarSet::new([].into_iter()));
            npc.pathing.tele = false;
            npc.pathing.walk_dir = 1;
            npc.pathing.run_dir = if nid % 3 == 0 { 4 } else { -1 };
            if nid % 3 == 1 {
                npc.pathing.move_speed = MoveSpeed::Crawl;
            }
            snap[nid as usize] = NpcSnapshot::new(&npc, renderer.highdefinitions(nid) as u16);
        }
        let (movement, updates) = info.encode(
            &mut renderer,
            &mut npcs,
            &snap,
            &ZoneMap::new(),
            &mut active,
            0,
            0,
            false,
        );
        assert!(movement.len() + updates.len() <= MAX_PACKET_SIZE - 3);
        let mut pos = 0;
        assert_eq!(bits(movement, &mut pos, 8), 250);
        let mut blocks = 0;
        for nid in 1..=250 {
            assert_eq!(bits(movement, &mut pos, 1), 1);
            if nid % 3 == 2 {
                assert_eq!(bits(movement, &mut pos, 2), 1);
                assert_eq!(bits(movement, &mut pos, 3), 0);
            } else {
                assert_eq!(bits(movement, &mut pos, 2), 2);
                assert_eq!(bits(movement, &mut pos, 1), u32::from(nid % 3 == 0));
                assert_eq!(bits(movement, &mut pos, 3), 0);
                if nid % 3 == 0 {
                    assert_eq!(bits(movement, &mut pos, 3), 2);
                }
            }
            blocks += bits(movement, &mut pos, 1) as usize;
        }
        assert_eq!(bits(movement, &mut pos, 15), 32767);
        assert_eq!((pos + 7) >> 3, movement.len());
        assert!(blocks > 0 && blocks < 250);
        assert_eq!(updates.len(), blocks * 1002);
    }

    struct Reader<'a> {
        b: &'a [u8],
        pos: usize,
    }

    impl Reader<'_> {
        fn byte(&mut self) -> i32 {
            self.pos += 1;
            self.b[self.pos - 1] as i8 as i32
        }
        fn g1(&mut self) -> i32 {
            self.byte() & 0xFF
        }
        fn g1_alt2(&mut self) -> i32 {
            -self.byte() & 0xFF
        }
        fn g1_alt3(&mut self) -> i32 {
            (128 - self.byte()) & 0xFF
        }
        fn g1_alt1(&mut self) -> i32 {
            (self.byte() - 128) & 0xFF
        }
        fn g2(&mut self) -> i32 {
            let hi = self.g1();
            (hi << 8) | self.g1()
        }
        fn g2_alt1(&mut self) -> i32 {
            let lo = self.g1();
            lo | (self.g1() << 8)
        }
        fn g2_alt2(&mut self) -> i32 {
            let hi = self.g1();
            let lo = (self.byte() - 128) & 0xFF;
            (hi << 8) | lo
        }
        fn g2_alt3(&mut self) -> i32 {
            let lo = (self.byte() - 128) & 0xFF;
            lo | (self.g1() << 8)
        }
        fn g4s_alt3(&mut self) -> i32 {
            let (a, b, c, d) = (self.g1(), self.g1(), self.g1(), self.g1());
            c | (b << 24) | (a << 16) | (d << 8)
        }
        fn gsmart(&mut self) -> i32 {
            if self.b[self.pos] < 128 {
                self.g1()
            } else {
                self.g2() - 32768
            }
        }
        fn gjstr(&mut self) -> String {
            let start = self.pos;
            while self.b[self.pos] != 0 {
                self.pos += 1;
            }
            self.pos += 1;
            String::from_utf8_lossy(&self.b[start..self.pos - 1]).into_owned()
        }
        fn hits(&mut self, count: i32) -> Vec<(i32, i32, i32, i32)> {
            (0..count)
                .map(|_| (self.gsmart(), self.gsmart(), self.gsmart(), self.g1()))
                .collect()
        }
        /// `Class244` mask 0x2 as the client reads each hit: (mark, amount,
        /// second mark, second amount, delay, bar), -1 where a part is absent.
        fn client_hits(&mut self, count: i32) -> Vec<(i32, i32, i32, i32, i32, i32)> {
            (0..count)
                .map(|_| {
                    let (mut mark, mut amount, mut mark2, mut amount2) = (-1, -1, -1, -1);
                    let first = self.gsmart();
                    if first == 32767 {
                        mark = self.gsmart();
                        amount = self.gsmart();
                        mark2 = self.gsmart();
                        amount2 = self.gsmart();
                    } else if first != 32766 {
                        mark = first;
                        amount = self.gsmart();
                    }
                    let delay = self.gsmart();
                    (mark, amount, mark2, amount2, delay, self.g1())
                })
                .collect()
        }
    }

    fn two_hits(info: &mut EntityMasks) {
        info.damage_type = Some(1);
        info.damage_taken = Some(25);
        info.damage_current = Some(75);
        info.damage_base = Some(100);
        info.damage2_type = Some(2);
        info.damage2_taken = Some(150);
        info.damage2_current = Some(60);
        info.damage2_base = Some(100);
    }

    #[test]
    fn appearance_blocks_read_back_through_the_client() {
        let mut active = observer();
        let mut renderer = PlayerRenderer::new();
        active.player.display_name = Some("Appearance".into());
        active.player.colours = [1, 2, 3, 4, 5];
        active.player.info.readyanim = Some(1426);
        active.player.info.masks = PlayerInfoProt::Appearance as u16;
        for gender in [0, 1] {
            active.player.gender = gender;
            for (headicons, skull, prayer) in [
                (0, -1, -1),
                (1, 0, -1),
                (8, -1, 0),
                (16, -1, 1),
                (32, -1, 2),
                (56, -1, 2),
                (1 | 16, 0, 1),
            ] {
                active.player.headicons = headicons;
                active.generateappearance(17);
                assert_eq!(active.player.info.last_appearance, Some(17));
                renderer.compute_info(1, &active.player.info, active.player.pathing.coord);
                let bytes = renderer.high_block(1);
                let mut r = Reader { b: bytes, pos: 0 };
                assert_eq!(r.g1(), 0x4);
                assert_eq!(r.g1_alt3() as usize, bytes.len() - 2);
                assert_eq!(r.g1(), gender as i32);
                assert_eq!((r.byte(), r.byte(), r.byte()), (-1, skull, prayer));
                assert_eq!(r.g1(), 0);
                for kit in [0, 0, 0, 0, 274, 0, 282, 292, 256, 289, 298, 266] {
                    let hi = r.g1();
                    let value = if hi == 0 { 0 } else { (hi << 8) | r.g1() };
                    assert_eq!(value, kit);
                }
                assert_eq!(r.g2(), 0);
                for colour in 1..=5 {
                    assert_eq!(r.g1(), colour);
                }
                assert_eq!(r.g2(), 1426);
                assert_eq!(r.gjstr(), "Appearance");
                assert_eq!(r.g1(), active.player.combat_level as i32);
                assert_eq!((r.g1(), r.g1(), r.g1()), (0, 255, 0));
                assert_eq!(r.pos, bytes.len());
                renderer.remove_temporary();
            }
        }
    }

    #[test]
    fn appearance_transmog_and_skill_level_read_back_through_the_client() {
        let mut active = observer();
        let mut renderer = PlayerRenderer::new();
        active.player.gender = 1;
        active.player.transmog = Some(4231);
        active.player.skill_level = 1500;
        active.player.display_name = Some("Transmog".into());
        active.player.info.masks = PlayerInfoProt::Appearance as u16;
        active.generateappearance(18);
        renderer.compute_info(1, &active.player.info, active.player.pathing.coord);
        let bytes = renderer.high_block(1);
        let mut r = Reader { b: bytes, pos: 0 };
        assert_eq!(r.g1(), 0x4);
        assert_eq!(r.g1_alt3() as usize, bytes.len() - 2);
        assert_eq!(r.g1(), 0x1 | 0x4);
        assert_eq!((r.byte(), r.byte(), r.byte(), r.g1()), (-1, -1, -1, 0));
        assert_eq!((r.g2(), r.g2(), r.g1()), (65535, 4231, 0));
        for _ in 0..5 {
            assert_eq!(r.g1(), 0);
        }
        assert_eq!(r.g2(), 1426);
        assert_eq!(r.gjstr(), "Transmog");
        assert_eq!(r.g1(), active.player.combat_level as i32);
        assert_eq!((r.g2(), r.g1()), (1500, 0));
        assert_eq!(r.pos, bytes.len());
    }

    #[test]
    fn npc_blocks_read_back_through_the_client() {
        let mut renderer = NpcRenderer::new();
        let mut info = EntityMasks::new();
        info.masks = NpcInfoProt::ChangeType as u16
            | NpcInfoProt::FaceCoord as u16
            | NpcInfoProt::SpotAnim as u16
            | NpcInfoProt::FaceEntity as u16
            | NpcInfoProt::Say as u16
            | NpcInfoProt::Damage as u16
            | NpcInfoProt::Damage2 as u16
            | NpcInfoProt::Anim as u16;
        info.changetype = Some(4231);
        info.face_x = Some(3222 * 2 + 1);
        info.face_z = Some(3218 * 2 + 1);
        info.spotanim = Some(74);
        info.spotanim_height = Some(92);
        info.spotanim_delay = Some(5);
        info.face_entity = Some(0x8000 + 12);
        info.say = Some("Moo".into());
        two_hits(&mut info);
        info.damage_source = Some(5);
        info.anim_id = Some(5577);
        info.anim_delay = Some(3);
        renderer.compute_info(9, &info);
        let mut npc_info = NpcInfo::new();
        renderer.write_high_block_for(&mut npc_info.updates, 9, 5);
        let bytes = &npc_info.updates.data[..npc_info.updates.pos];

        let mut r = Reader { b: bytes, pos: 0 };
        let mut mask = r.g1();
        if mask & 0x8 != 0 {
            mask += r.g1() << 8;
        }
        if mask & 0x100 != 0 {
            mask += r.g1() << 16;
        }
        assert_eq!(
            mask,
            0x2 | 0x4 | 0x20 | 0x10 | 0x40 | 0x1 | 0x80,
            "mask bits"
        );

        assert_eq!(r.g2(), 4231, "0x2 change type");
        assert_eq!(
            (r.g2(), r.g2_alt1()),
            (3222 * 2 + 1, 3218 * 2 + 1),
            "0x4 face coord, fine"
        );
        assert_eq!(r.g2_alt3(), 74, "0x20 spotanim id");
        assert_eq!(r.g4s_alt3(), 92 << 16 | 5, "0x20 spotanim height and delay");
        assert_eq!(r.g1_alt1() >> 3 & 0xF, 15, "0x20 no attachment");
        assert_eq!(r.g2_alt1(), 0x8000 + 12, "0x10 face entity");
        assert_eq!(r.gjstr(), "Moo", "0x40 say");
        let count = r.g1_alt1();
        assert_eq!(
            r.hits(count),
            vec![(1, 25, 0, 75 * 255 / 100), (2 + 14, 150, 0, 60 * 255 / 100)],
            "0x1 both hits in one block: player 5's bright, the other darker"
        );
        for channel in 0..4 {
            assert_eq!(r.g2_alt2(), 5577, "0x80 anim channel {channel}");
        }
        assert_eq!(r.g1_alt2(), 3, "0x80 anim delay");
        assert_eq!(r.pos, bytes.len(), "the client reads every byte written");
    }

    #[test]
    fn player_blocks_read_back_through_the_client() {
        let mut renderer = PlayerRenderer::new();
        let mut info = EntityMasks::new();
        info.masks = PlayerInfoProt::Anim as u16
            | PlayerInfoProt::Damage as u16
            | PlayerInfoProt::Damage2 as u16
            | PlayerInfoProt::FaceEntity as u16
            | PlayerInfoProt::SpotAnim as u16
            | PlayerInfoProt::Say as u16;
        info.anim_id = Some(879);
        info.anim_delay = Some(3);
        two_hits(&mut info);
        info.face_entity = Some(42);
        info.spotanim = Some(90);
        info.spotanim_height = Some(100);
        info.spotanim_delay = Some(30);
        info.say = Some("Hi".into());
        renderer.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        let mut player_info = PlayerInfo::new();
        renderer.write_high_block_for(&mut player_info.updates, 7, Observer { pid: 7 });
        let bytes = &player_info.updates.data[..player_info.updates.pos];

        let mut r = Reader { b: bytes, pos: 0 };
        let mut mask = r.g1();
        if mask & 0x20 != 0 {
            mask += r.g1() << 8;
        }
        if mask & 0x8000 != 0 {
            mask += r.g1() << 16;
        }
        assert_eq!(mask & !0x20, 0x80 | 0x2 | 0x10 | 0x40 | 0x200, "mask bits");

        for channel in 0..4 {
            assert_eq!(r.g2(), 879, "0x80 anim channel {channel}");
        }
        assert_eq!(r.g1_alt1(), 3, "0x80 anim delay");
        let count = r.g1();
        assert_eq!(
            r.hits(count),
            vec![(1, 25, 0, 75 * 255 / 100), (2, 150, 0, 60 * 255 / 100)],
            "0x2 both hits in one block"
        );
        assert_eq!(r.g2_alt3(), 42, "0x10 face entity");
        assert_eq!(r.g2_alt2(), 90, "0x40 spotanim id");
        assert_eq!(
            r.g4s_alt3(),
            100 << 16 | 30,
            "0x40 spotanim height and delay"
        );
        assert_eq!(r.g1_alt3() >> 3 & 0xF, 15, "0x40 no attachment");
        assert_eq!(r.gjstr(), "Hi", "0x200 say");
        assert_eq!(r.pos, bytes.len(), "the client reads every byte written");
    }

    /// The 647 client reads an exact move's start and end as signed tile
    /// offsets from the player's own tile (`Class244`, mask 0x4000: added to
    /// `pathX[0]`/`pathZ[0]`, where the move has already put him), not as
    /// world coordinates: 3079,9786 sent as a byte read back as an offset
    /// of (7, 58) tiles, off the map, and the client's off-map reset dropped
    /// the player's animation (the coffin cutscene's throw).
    #[test]
    fn an_exact_move_reads_back_relative_to_the_players_tile() {
        let mut renderer = PlayerRenderer::new();
        let mut info = EntityMasks::new();
        info.masks = PlayerInfoProt::ExactMove as u16;
        info.exactmove_start_x = Some(3079);
        info.exactmove_start_z = Some(9786);
        info.exactmove_end_x = Some(3078);
        info.exactmove_end_z = Some(9786);
        info.exactmove_begin = Some(0);
        info.exactmove_finish = Some(30);
        info.exactmove_dir = Some(1);
        renderer.compute_info(7, &info, CoordGrid::new(3078, 0, 9786));
        let mut player_info = PlayerInfo::new();
        renderer.write_high_block_for(&mut player_info.updates, 7, Observer::default());
        let bytes = &player_info.updates.data[..player_info.updates.pos];

        let mut r = Reader { b: bytes, pos: 0 };
        let mut mask = r.g1();
        if mask & 0x20 != 0 {
            mask += r.g1() << 8;
        }
        assert_eq!(mask & !0x20, 0x4000, "the exact move mask alone");
        // g1b_alt2, g1b, g1b_alt1, g1b_alt2: the client's signed reads.
        let start_x = (128 - r.byte()) as i8;
        let start_z = r.byte() as i8;
        let end_x = (-r.byte()) as i8;
        let end_z = (128 - r.byte()) as i8;
        assert_eq!(
            (start_x, start_z),
            (1, 0),
            "starts a tile east of where he is"
        );
        assert_eq!((end_x, end_z), (0, 0), "ends where he is");
        assert_eq!(r.g2_alt2(), 0, "begin");
        assert_eq!(r.g2(), 30, "finish");
        assert_eq!(r.g1_alt2(), 1, "direction");
        assert_eq!(r.pos, bytes.len(), "the client reads every byte written");
    }

    #[test]
    fn a_new_observer_receives_the_players_persistent_facing() {
        let mut renderer = PlayerRenderer::new();
        let mut active = observer();
        let mut other = observer();
        other.player.uid = PlayerUid::new("other".into(), 7);
        other.player.info.last_appearance = Some(1);
        other.player.info.last_appearance_info = Some(vec![1, 2, 3].into_boxed_slice());
        other.player.info.masks = PlayerInfoProt::Appearance as u16;
        renderer.compute_info(7, &other.player.info, other.player.pathing.coord);
        renderer.remove_temporary();
        other.player.info.masks = 0;
        other.player.info.face_entity = Some(42);
        other.player.info.orientation_x = Some(CoordGrid::fine(3223, 1));
        other.player.info.orientation_z = Some(CoordGrid::fine(3218, 1));

        let mut info = PlayerInfo::new();
        renderer.cache_low(7, &other.player.info, other.player.pathing.coord);
        renderer.ready_low(7);
        let appearance = PlayerInfo::low_appearance(&renderer, &mut active, 7);
        renderer.write_low_block_for(&mut info.updates, 7, Observer::default(), appearance);
        let mut r = Reader {
            b: &info.updates.data[..info.updates.pos],
            pos: 0,
        };
        assert_eq!(r.g1(), 0x1 | 0x4 | 0x8 | 0x10);
        assert_eq!(r.g2_alt2(), 0x3000);
        assert_eq!(r.g1(), 1);
        assert_eq!(r.g2_alt3(), 42);
        assert_eq!(&r.b[r.pos..], &[125, 1, 2, 3]);
        assert!(renderer.high_block(7).is_empty());
    }

    #[test]
    fn a_life_point_hit_past_255_reads_back_whole() {
        let mut renderer = PlayerRenderer::new();
        let mut info = EntityMasks::new();
        info.apply_damage(300, 3, 128, u8::MAX, PlayerInfoProt::Damage as u16);
        renderer.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        let mut player_info = PlayerInfo::new();
        renderer.write_high_block_for(&mut player_info.updates, 7, Observer { pid: 7 });
        let bytes = &player_info.updates.data[..player_info.updates.pos];

        let mut r = Reader { b: bytes, pos: 0 };
        assert_eq!(r.g1(), 0x2, "the hits mask alone");
        let count = r.g1();
        assert_eq!(r.hits(count), vec![(3, 300, 0, 128)]);
        assert_eq!(r.pos, bytes.len(), "the client reads every byte written");

        renderer.remove_temporary();
        info.masks = PlayerInfoProt::Damage2 as u16;
        info.damage2_taken = info.damage_taken;
        info.damage2_type = info.damage_type;
        info.damage2_current = info.damage_current;
        info.damage2_base = info.damage_base;
        renderer.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        player_info.updates.pos = 0;
        // A bystander's view is the shared block.
        renderer.write_high_block_for(&mut player_info.updates, 7, Observer { pid: 9 });
        assert_eq!(
            &player_info.updates.data[..player_info.updates.pos],
            renderer.high_block(7),
        );
    }

    /// 2011's soaked hit is one hitsplat with two marks, the hit's and the
    /// blue soak mark (5) beside it, as the client reads them. Two soaked hits
    /// in a tick are the largest hit block, and both read back whole.
    #[test]
    fn a_soaked_hit_reads_back_as_one_double_hitsplat() {
        let mut renderer = PlayerRenderer::new();
        let mut info = EntityMasks::new();
        info.apply_damage(375, 1, 158, u8::MAX, PlayerInfoProt::Damage as u16);
        info.damage_soak = Some(75);
        info.masks |= PlayerInfoProt::Damage2 as u16;
        info.damage2_taken = Some(32000);
        info.damage2_type = Some(10);
        info.damage2_current = Some(0);
        info.damage2_base = Some(u8::MAX);
        info.damage2_soak = Some(30000);
        renderer.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        let mut player_info = PlayerInfo::new();
        renderer.write_high_block_for(&mut player_info.updates, 7, Observer { pid: 7 });
        let bytes = &player_info.updates.data[..player_info.updates.pos];

        let mut r = Reader { b: bytes, pos: 0 };
        assert_eq!(r.g1(), 0x2, "the hits mask alone");
        let count = r.g1();
        assert_eq!(
            r.client_hits(count),
            vec![(1, 375, 5, 75, 0, 158), (10, 32000, 5, 30000, 0, 0)]
        );
        assert_eq!(r.pos, bytes.len(), "the client reads every byte written");

        // Without a soak the hit is the single form it always was.
        renderer.remove_temporary();
        let mut info = EntityMasks::new();
        info.apply_damage(150, 0, 200, u8::MAX, PlayerInfoProt::Damage as u16);
        renderer.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        player_info.updates.pos = 0;
        renderer.write_high_block_for(&mut player_info.updates, 7, Observer { pid: 7 });
        let bytes = &player_info.updates.data[..player_info.updates.pos];
        let mut r = Reader { b: bytes, pos: 0 };
        assert_eq!(r.g1(), 0x2);
        let count = r.g1();
        assert_eq!(r.client_hits(count), vec![(0, 150, -1, -1, 0, 200)]);
        assert_eq!(r.pos, bytes.len());
    }

    /// 2011 paints a hit bright for its receiver and its dealer, darker
    /// (hitmark + 14, cache 786's 14-27) for everyone else, who sees no splat
    /// at all for a hit of nothing, only the health bar; a heal is never
    /// darkened, and a bystander's soak is the darker soak (19) (2011 wiki,
    /// Hitsplat, 2011-04-13; void-634 HitSplat). Player 9 hits player 7 for a
    /// soaked 300; a miss from an npc lands with it.
    #[test]
    fn hits_are_bright_for_their_dealer_and_receiver_and_darker_for_others() {
        let mut renderer = PlayerRenderer::new();
        let mut info = EntityMasks::new();
        info.masks = PlayerInfoProt::Anim as u16
            | PlayerInfoProt::Damage as u16
            | PlayerInfoProt::Damage2 as u16;
        info.anim_id = Some(424);
        info.anim_delay = Some(0);
        info.damage_type = Some(1);
        info.damage_taken = Some(300);
        info.damage_current = Some(200);
        info.damage_base = Some(u8::MAX);
        info.damage_soak = Some(50);
        info.damage_source = Some(9);
        info.damage2_type = Some(8);
        info.damage2_taken = Some(0);
        info.damage2_current = Some(200);
        info.damage2_base = Some(u8::MAX);
        renderer.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));

        let seen = |observer: u16| {
            let mut player_info = PlayerInfo::new();
            renderer.write_high_block_for(&mut player_info.updates, 7, Observer { pid: observer });
            let bytes = player_info.updates.data[..player_info.updates.pos].to_vec();
            let mut r = Reader { b: &bytes, pos: 0 };
            let mask = r.g1();
            assert_eq!(mask, 0x80 | 0x2);
            for _ in 0..4 {
                r.g2();
            }
            r.g1_alt1();
            let count = r.g1();
            let hits = r.client_hits(count);
            assert_eq!(r.pos, bytes.len(), "{observer} reads every byte written");
            (hits, bytes.len())
        };
        let (taker, len) = seen(7);
        assert_eq!(
            taker,
            vec![(1, 300, 5, 50, 0, 200), (8, 0, -1, -1, 0, 200)],
            "the receiver sees both bright"
        );
        let (dealer, dealer_len) = seen(9);
        assert_eq!(
            dealer,
            vec![(1, 300, 5, 50, 0, 200), (-1, -1, -1, -1, 0, 200)],
            "the dealer sees his hit bright, the npc's miss not at all"
        );
        let (bystander, bystander_len) = seen(11);
        assert_eq!(
            bystander,
            vec![(15, 300, 19, 50, 0, 200), (-1, -1, -1, -1, 0, 200)],
            "a bystander sees the hit and its soak darker, the miss not at all"
        );
        assert_eq!(
            (dealer_len, bystander_len),
            (len, len),
            "every view is one length"
        );

        // A heal keeps its splat for everyone.
        renderer.remove_temporary();
        let mut info = EntityMasks::new();
        info.apply_damage(40, 9, 255, u8::MAX, PlayerInfoProt::Damage as u16);
        renderer.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        let mut player_info = PlayerInfo::new();
        renderer.write_high_block_for(&mut player_info.updates, 7, Observer { pid: 11 });
        let bytes = &player_info.updates.data[..player_info.updates.pos];
        let mut r = Reader { b: bytes, pos: 0 };
        assert_eq!(r.g1(), 0x2);
        let count = r.g1();
        assert_eq!(r.client_hits(count), vec![(9, 40, -1, -1, 0, 255)]);
    }
}
