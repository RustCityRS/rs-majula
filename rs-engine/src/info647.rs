//! Rev-647 player/NPC info encoders.
//!
//! Player info is the 647 4-pass NSN-flag protocol (client decoder
//! Class357.method2263): two high-definition passes over tracked (local)
//! slots and two low-definition passes over the remaining (external) slots,
//! partitioned by the per-slot NSN flag that alternates which pass examines
//! a slot each tick. Skip counts compress runs of untouched slots. NPC info
//! keeps the tracked-walk + nearby-adds shape of earlier revisions with the
//! 647 bit layouts (46-bit adds, N=0 direction map, u32 mask headers).
//!
//! The per-tick [`PlayerSnapshot`]/[`NpcSnapshot`] structs and their flag
//! semantics are shared with the jag-era encoder; movement decisions read
//! the snapshots (value-identical to the live structs at output time).

use crate::active_npc::ActiveNpc;
use crate::active_player::ActivePlayer;
use crate::build::ActiveBuildArea;
use rs_entity::build::BuildArea;
use rs_entity::dir_delta;
use rs_grid::CoordGrid;
use rs_info::{NPC_MASK_EXTENDED, NPC_MASK_EXTENDED2, NpcInfoProt647, PlayerInfoProt647};
use rs_info::{NpcRenderer, PlayerRenderer};
use rs_io::Packet;
use rs_io::packet::BitWriter;
use rs_zone::zone_map::ZoneMap;

/// Compact per-tick snapshot of the player fields the encoders read, indexed
/// by pid. Same layout and flag semantics as the jag-era module.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct PlayerSnapshot {
    pub coord: u32,
    pub len: u16,
    pub run_dir: i8,
    pub walk_dir: i8,
    pub flags: u8,
}

impl PlayerSnapshot {
    pub const PRESENT: u8 = 1 << 0;
    pub const ACTIVE: u8 = 1 << 1;
    pub const TELE: u8 = 1 << 2;
    pub const VIS_HARD: u8 = 1 << 3;
    pub const HAS_EXACTMOVE: u8 = 1 << 4;

    pub const ABSENT: PlayerSnapshot = PlayerSnapshot {
        coord: 0,
        len: 0,
        run_dir: -1,
        walk_dir: -1,
        flags: 0,
    };

    /// Clears the `PRESENT` bit, mirroring a `players[pid].take()` / removal.
    #[inline(always)]
    pub const fn clear(&mut self) {
        self.flags = 0;
    }

    /// Whether a tracked player leaves the observer's view this tick.
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

/// Compact per-tick snapshot of the NPC fields the encoders read, indexed by
/// nid. Same layout and flag semantics as the jag-era module.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct NpcSnapshot {
    pub coord: u32,
    pub len: u16,
    pub run_dir: i8,
    pub walk_dir: i8,
    pub flags: u8,
}

impl NpcSnapshot {
    pub const PRESENT: u8 = 1 << 0;
    pub const ACTIVE: u8 = 1 << 1;
    pub const TELE: u8 = 1 << 2;

    pub const ABSENT: NpcSnapshot = NpcSnapshot {
        coord: 0,
        len: 0,
        run_dir: -1,
        walk_dir: -1,
        flags: 0,
    };

    #[inline(always)]
    pub const fn clear(&mut self) {
        self.flags = 0;
    }

    /// Whether a tracked NPC leaves the observer's view this tick.
    #[inline(always)]
    pub fn should_remove(&self, obs_coord: CoordGrid, obs_y: u8, view_distance: u8) -> bool {
        self.flags & NpcSnapshot::PRESENT == 0
            || self.flags & NpcSnapshot::TELE != 0
            || CoordGrid::from(self.coord).y() != obs_y
            || !CoordGrid::in_distance(&obs_coord, CoordGrid::from(self.coord), view_distance)
            || self.flags & NpcSnapshot::ACTIVE == 0
    }
}

const MAX_LOCAL_PLAYERS: usize = 255;
const BYTES_LIMIT: usize = u16::MAX as usize;

// Server: NW=0 N=1 NE=2 W=3 E=4 SW=5 S=6 SE=7
// Player client: SW=0 S=1 SE=2 W=3 E=4 NW=5 N=6 NE=7
const WALK_DIR_MAP: [i32; 8] = [5, 6, 7, 3, 4, 0, 1, 2];
// NPC client: N=0 NE=1 E=2 SE=3 S=4 SW=5 W=6 NW=7
const NPC_DIR_MAP: [i32; 8] = [7, 0, 1, 6, 2, 5, 4, 3];

/// Pad the bit stream to a byte boundary between GPI passes (the client
/// aligns its read cursor the same way).
fn align_byte(bw: &mut BitWriter, buf: &mut Packet) {
    match bw.bitpos() & 7 {
        1 => bw.pbit::<7>(buf, 0),
        2 => bw.pbit::<6>(buf, 0),
        3 => bw.pbit::<5>(buf, 0),
        4 => bw.pbit::<4>(buf, 0),
        5 => bw.pbit::<3>(buf, 0),
        6 => bw.pbit::<2>(buf, 0),
        7 => bw.pbit::<1>(buf, 0),
        _ => {}
    }
}

/// Skip-count encoding (client Class272.method1773).
fn encode_skip(bw: &mut BitWriter, buf: &mut Packet, count: usize) {
    if count == 0 {
        bw.pbit::<2>(buf, 0);
    } else if count < 32 {
        bw.pbit::<2>(buf, 1);
        bw.pbit::<5>(buf, count as i32);
    } else if count < 256 {
        bw.pbit::<2>(buf, 2);
        bw.pbit::<8>(buf, count as i32);
    } else {
        bw.pbit::<2>(buf, 3);
        bw.pbit::<11>(buf, count as i32);
    }
}

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
        let dir = match (dx, dz) {
            (-1, -1) => 0,
            (0, -1) => 1,
            (1, -1) => 2,
            (-1, 0) => 3,
            (1, 0) => 4,
            (-1, 1) => 5,
            (0, 1) => 6,
            (1, 1) => 7,
            _ => 6,
        };
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

fn walk_index(dx: i32, dz: i32) -> i32 {
    match (dx.signum(), dz.signum()) {
        (-1, 1) => 0,
        (0, 1) => 1,
        (1, 1) => 2,
        (-1, 0) => 3,
        (1, 0) => 4,
        (-1, -1) => 5,
        (0, -1) => 6,
        (1, -1) => 7,
        _ => 0,
    }
}

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

fn within_distance(a: CoordGrid, b: CoordGrid, dist: u16) -> bool {
    if a.y() != b.y() {
        return false;
    }
    let dx = (a.x() as i32 - b.x() as i32).abs();
    let dz = (a.z() as i32 - b.z() as i32).abs();
    dx <= dist as i32 && dz <= dist as i32
}

/// Encodes the rev-647 player info packet for a single observer per tick.
pub struct PlayerInfo {
    buf: Packet,
    blocks: Packet,
    bw: BitWriter,
}

impl PlayerInfo {
    #[inline(always)]
    pub fn new() -> PlayerInfo {
        PlayerInfo {
            buf: Packet::new(BYTES_LIMIT),
            blocks: Packet::new(BYTES_LIMIT),
            bw: BitWriter::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn encode(
        &mut self,
        renderer: &mut PlayerRenderer,
        _players: crate::engine::OtherPlayers<'_>,
        snap: &[PlayerSnapshot],
        _map: &ZoneMap,
        active: &mut ActivePlayer,
        dx: i32,
        dz: i32,
        rebuild: bool,
    ) -> &[u8] {
        let self_pid = active.player.uid.pid();
        let self_coord = active.player.pathing.coord;
        let build = &mut active.player.build_area;

        // The 647 protocol is global-slot: the tracked set survives map
        // rebuilds and out-of-range players fall out through the high-def
        // remove path, so only the view-distance resize runs here.
        let _ = (dx, dz, rebuild);
        build.resize();
        let view_dist = build.view_distance as i32;

        self.blocks.pos = 0;
        self.bw.reset();

        let mut additions: usize = 0;

        // Pass 1: high-def (local), nsn flag NOT set
        self.high_definition(
            snap, renderer, active, self_pid, self_coord, true, view_dist,
        );
        align_byte(&mut self.bw, &mut self.buf);

        // Pass 2: high-def (local), nsn flag IS set
        self.high_definition(
            snap, renderer, active, self_pid, self_coord, false, view_dist,
        );
        align_byte(&mut self.bw, &mut self.buf);

        // Pass 3: low-def (external), nsn flag IS set
        self.low_definition(
            snap,
            renderer,
            active,
            self_pid,
            self_coord,
            true,
            &mut additions,
            view_dist,
        );
        align_byte(&mut self.bw, &mut self.buf);

        // Pass 4: low-def (external), nsn flag NOT set
        self.low_definition(
            snap,
            renderer,
            active,
            self_pid,
            self_coord,
            false,
            &mut additions,
            view_dist,
        );
        align_byte(&mut self.bw, &mut self.buf);

        self.bw.finish(&mut self.buf);
        if self.blocks.pos > 0 {
            self.buf.pdata(&self.blocks.data, 0, self.blocks.pos);
        }

        // Cycle NSN flags for next tick.
        active.player.build_area.cycle_nsn_flags();

        unsafe { self.buf.data.get_unchecked(0..self.buf.pos) }
    }

    // ── High definition (local players) ──────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    fn high_definition(
        &mut self,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        self_pid: u16,
        self_coord: CoordGrid,
        nsn: bool,
        view_dist: i32,
    ) {
        let mut skip: i32 = -1;
        let local_count = observer.player.build_area.local_player_indices.len();

        for i in 0..local_count {
            let index = observer.player.build_area.local_player_indices[i] as usize;
            let flag_set = (observer.player.build_area.nsn_flags[index] & 0x1) != 0;
            if nsn == flag_set {
                continue;
            }

            let is_self = index as u16 == self_pid;

            let (removing, updating, moved) = if is_self {
                let has_masks = renderer.highdefinitions(self_pid) > 0;
                let moved = observer.player.pathing.tele || observer.player.pathing.walk_dir != -1;
                (false, has_masks, moved)
            } else {
                let s = unsafe { *snap.get_unchecked(index) };
                if s.flags & PlayerSnapshot::PRESENT != 0 {
                    let removing =
                        !within_distance(CoordGrid::from(s.coord), self_coord, view_dist as u16)
                            || s.flags & PlayerSnapshot::ACTIVE == 0
                            || s.flags & PlayerSnapshot::VIS_HARD != 0;
                    let has_masks = renderer.highdefinitions(index as u16) > 0;
                    let moved = s.flags & PlayerSnapshot::TELE != 0 || s.walk_dir != -1;
                    (removing, has_masks, moved)
                } else {
                    (true, false, false)
                }
            };

            let active = removing || updating || moved;

            if !active {
                skip += 1;
                observer.player.build_area.nsn_flags[index] |= 2;
                continue;
            }

            if skip >= 0 {
                self.bw.pbit::<1>(&mut self.buf, 0);
                encode_skip(&mut self.bw, &mut self.buf, skip as usize);
                skip = -1;
            }

            self.bw.pbit::<1>(&mut self.buf, 1);
            self.process_high_def(snap, renderer, observer, index, is_self, removing);
        }

        if skip >= 0 {
            self.bw.pbit::<1>(&mut self.buf, 0);
            encode_skip(&mut self.bw, &mut self.buf, skip as usize);
        }
    }

    fn process_high_def(
        &mut self,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        index: usize,
        is_self: bool,
        removing: bool,
    ) {
        if removing {
            self.bw.pbit::<1>(&mut self.buf, 0); // no mask blocks
            self.bw.pbit::<2>(&mut self.buf, 0); // type = remove
            self.validate_coordinates(snap, observer, index);
            observer.player.build_area.players.remove(index as u16);
            return;
        }

        let (tele, walk_dir, run_dir, coord, jump) = if is_self {
            (
                observer.player.pathing.tele,
                observer.player.pathing.walk_dir,
                observer.player.pathing.run_dir,
                observer.player.pathing.coord,
                observer.player.pathing.jump,
            )
        } else {
            let s = unsafe { *snap.get_unchecked(index) };
            if s.flags & PlayerSnapshot::PRESENT == 0 {
                self.bw.pbit::<1>(&mut self.buf, 0);
                self.bw.pbit::<2>(&mut self.buf, 0);
                self.validate_coordinates(snap, observer, index);
                observer.player.build_area.players.remove(index as u16);
                return;
            }
            (
                s.flags & PlayerSnapshot::TELE != 0,
                s.walk_dir,
                s.run_dir,
                CoordGrid::from(s.coord),
                false,
            )
        };
        let _ = jump;

        let has_blocks = renderer.highdefinitions(index as u16) > 0;

        if tele {
            let last = observer.player.build_area.player_last_coords[index];
            let dx = coord.x() as i32 - last.x() as i32;
            let dz = coord.z() as i32 - last.z() as i32;
            let dl = coord.y() as i32 - last.y() as i32;
            let local = dx.abs() <= 15 && dz.abs() <= 15;

            self.bw.pbit::<1>(&mut self.buf, has_blocks as i32);
            self.bw.pbit::<2>(&mut self.buf, 3); // teleport
            if local {
                self.bw.pbit::<1>(&mut self.buf, 0);
                self.bw.pbit::<12>(
                    &mut self.buf,
                    ((dl as u32 & 0x3) << 10) as i32
                        | ((dx as u32 & 0x1F) << 5) as i32
                        | (dz as u32 & 0x1F) as i32,
                );
            } else {
                self.bw.pbit::<1>(&mut self.buf, 1);
                self.bw.pbit::<30>(
                    &mut self.buf,
                    ((dl as u32 & 0x3) << 28) as i32
                        | ((dx as u32 & 0x3FFF) << 14) as i32
                        | (dz as u32 & 0x3FFF) as i32,
                );
            }

            if has_blocks {
                self.write_high_blocks(renderer, index as u16);
            }
            observer.player.build_area.player_last_coords[index] = coord;
        } else if run_dir != -1 {
            // The 4-bit run encoding only covers the 16 combined deltas at
            // Chebyshev distance 2. When two steps produce a closer delta
            // (e.g. a corner cut), fall back to walk encoding with the
            // combined direction — run speed still applies via MoveType.
            let (wx, wz) = dir_delta(walk_dir);
            let (rx, rz) = dir_delta(run_dir);
            let dx = wx as i32 + rx as i32;
            let dz = wz as i32 + rz as i32;
            self.bw.pbit::<1>(&mut self.buf, has_blocks as i32);
            if let Some(ri) = run_index(dx, dz) {
                self.bw.pbit::<2>(&mut self.buf, 2); // run
                self.bw.pbit::<4>(&mut self.buf, ri);
            } else {
                self.bw.pbit::<2>(&mut self.buf, 1); // walk
                self.bw
                    .pbit::<3>(&mut self.buf, WALK_DIR_MAP[walk_index(dx, dz) as usize]);
            }
            if has_blocks {
                self.write_high_blocks(renderer, index as u16);
            }
            observer.player.build_area.player_last_coords[index] = coord;
        } else if walk_dir != -1 {
            self.bw.pbit::<1>(&mut self.buf, has_blocks as i32);
            self.bw.pbit::<2>(&mut self.buf, 1); // walk
            self.bw
                .pbit::<3>(&mut self.buf, WALK_DIR_MAP[walk_dir as usize]);
            if has_blocks {
                self.write_high_blocks(renderer, index as u16);
            }
            observer.player.build_area.player_last_coords[index] = coord;
        } else if has_blocks {
            self.bw.pbit::<1>(&mut self.buf, 1);
            self.bw.pbit::<2>(&mut self.buf, 0);
            self.write_high_blocks(renderer, index as u16);
        }
    }

    // ── Low definition (external players) ────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    fn low_definition(
        &mut self,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        self_pid: u16,
        self_coord: CoordGrid,
        nsn: bool,
        additions: &mut usize,
        view_dist: i32,
    ) {
        let mut skip: i32 = -1;
        let external_count = observer.player.build_area.external_player_indices.len();
        let local_count = observer.player.build_area.local_player_indices.len();

        for i in 0..external_count {
            let index = observer.player.build_area.external_player_indices[i] as usize;
            let flag_clear = (observer.player.build_area.nsn_flags[index] & 0x1) == 0;
            if nsn == flag_clear {
                continue;
            }

            let s = unsafe { *snap.get_unchecked(index) };
            let adding = s.flags & PlayerSnapshot::PRESENT != 0
                && (local_count + *additions) < MAX_LOCAL_PLAYERS
                && index as u16 != self_pid
                && s.flags & PlayerSnapshot::ACTIVE != 0
                && within_distance(CoordGrid::from(s.coord), self_coord, view_dist as u16);

            if !adding {
                skip += 1;
                observer.player.build_area.nsn_flags[index] |= 2;
                continue;
            }

            if skip >= 0 {
                self.bw.pbit::<1>(&mut self.buf, 0);
                encode_skip(&mut self.bw, &mut self.buf, skip as usize);
                skip = -1;
            }

            self.bw.pbit::<1>(&mut self.buf, 1);
            self.process_low_def(snap, renderer, observer, index);
            *additions += 1;
        }

        if skip >= 0 {
            self.bw.pbit::<1>(&mut self.buf, 0);
            encode_skip(&mut self.bw, &mut self.buf, skip as usize);
        }
    }

    fn process_low_def(
        &mut self,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        index: usize,
    ) {
        let coord = CoordGrid::from(unsafe { *snap.get_unchecked(index) }.coord);

        self.bw.pbit::<2>(&mut self.buf, 0); // state = spawn
        self.validate_coordinates(snap, observer, index);
        self.bw.pbit::<6>(&mut self.buf, (coord.x() & 0x3F) as i32);
        self.bw.pbit::<6>(&mut self.buf, (coord.z() & 0x3F) as i32);
        self.bw.pbit::<1>(&mut self.buf, 1); // has blocks

        self.write_low_blocks(renderer, index as u16);

        observer.player.build_area.players.insert(index as u16);
        observer.player.build_area.player_last_coords[index] = coord;
        observer.player.build_area.nsn_flags[index] |= 2;
    }

    // ── Coordinate validation (used on both add and remove) ──────────────

    fn validate_coordinates(
        &mut self,
        snap: &[PlayerSnapshot],
        observer: &mut ActivePlayer,
        index: usize,
    ) {
        let current_packed = observer.player.build_area.player_mapsquares[index];
        let s = unsafe { *snap.get_unchecked(index) };
        let new_packed = if s.flags & PlayerSnapshot::PRESENT != 0 {
            BuildArea::pack_mapsquare(CoordGrid::from(s.coord))
        } else {
            current_packed
        };
        let updating = new_packed != current_packed;

        self.bw.pbit::<1>(&mut self.buf, updating as i32);
        if updating {
            update_coordinates(&mut self.bw, &mut self.buf, current_packed, new_packed);
            observer.player.build_area.player_mapsquares[index] = new_packed;
        }
    }

    // ── Mask block encoding ─────────────────────────────────────────────

    fn write_high_blocks(&mut self, renderer: &mut PlayerRenderer, pid: u16) {
        let masks = Self::collect_masks(renderer, pid);
        write_mask_header(&mut self.blocks, masks);
        write_mask_data(&mut self.blocks, renderer, pid, masks);
    }

    fn write_low_blocks(&mut self, renderer: &mut PlayerRenderer, pid: u16) {
        let mut masks = Self::collect_masks(renderer, pid);
        // New spawns always carry Appearance, FaceCoord and MoveType so the
        // client can render them. Prime an idle FaceCoord (angle 0) if none is
        // cached.
        if renderer.has(pid, PlayerInfoProt647::Appearance) {
            masks |= PlayerInfoProt647::Appearance as u32;
        }
        if !renderer.has(pid, PlayerInfoProt647::FaceCoord) {
            renderer.cache_face_coord(pid, 0);
        }
        masks |= PlayerInfoProt647::FaceCoord as u32;
        // And their move type: it is only sent when it changes, so an observer
        // adding them later has never seen it.
        renderer.cache_move_type(pid);
        masks |= PlayerInfoProt647::MoveType as u32;
        write_mask_header(&mut self.blocks, masks);
        write_mask_data(&mut self.blocks, renderer, pid, masks);
    }

    fn collect_masks(renderer: &PlayerRenderer, pid: u16) -> u32 {
        let mut masks: u32 = 0;
        for prot in [
            PlayerInfoProt647::FaceCoord,
            PlayerInfoProt647::TempMoveType,
            PlayerInfoProt647::Anim,
            PlayerInfoProt647::MoveType,
            PlayerInfoProt647::ExactMove,
            PlayerInfoProt647::Hit,
            PlayerInfoProt647::FaceEntity,
            PlayerInfoProt647::SpotAnim,
            PlayerInfoProt647::Say,
            PlayerInfoProt647::Appearance,
        ] {
            if renderer.has(pid, prot) {
                masks |= prot as u32;
            }
        }
        masks
    }
}

fn write_mask_header(buf: &mut Packet, mut masks: u32) {
    if masks == 0 {
        buf.p1(0);
        return;
    }
    const EXTENDED: u32 = 0x20;
    const EXTENDED2: u32 = 0x8000;
    if (masks >> 8) != 0 {
        masks |= EXTENDED;
    }
    if (masks >> 16) != 0 {
        masks |= EXTENDED2;
    }
    buf.p1(masks as u8);
    if masks & EXTENDED != 0 {
        buf.p1((masks >> 8) as u8);
        if masks & EXTENDED2 != 0 {
            buf.p1((masks >> 16) as u8);
        }
    }
}

fn write_mask_data(buf: &mut Packet, renderer: &mut PlayerRenderer, pid: u16, masks: u32) {
    // Client decode order (Class244.method1590).
    if masks & PlayerInfoProt647::FaceCoord as u32 != 0 {
        renderer.write(buf, pid, PlayerInfoProt647::FaceCoord);
    }
    if masks & PlayerInfoProt647::TempMoveType as u32 != 0 {
        renderer.write(buf, pid, PlayerInfoProt647::TempMoveType);
    }
    if masks & PlayerInfoProt647::Anim as u32 != 0 {
        renderer.write(buf, pid, PlayerInfoProt647::Anim);
    }
    if masks & PlayerInfoProt647::MoveType as u32 != 0 {
        renderer.write(buf, pid, PlayerInfoProt647::MoveType);
    }
    if masks & PlayerInfoProt647::ExactMove as u32 != 0 {
        renderer.write(buf, pid, PlayerInfoProt647::ExactMove);
    }
    if masks & PlayerInfoProt647::Hit as u32 != 0 {
        renderer.write(buf, pid, PlayerInfoProt647::Hit);
    }
    if masks & PlayerInfoProt647::FaceEntity as u32 != 0 {
        renderer.write(buf, pid, PlayerInfoProt647::FaceEntity);
    }
    if masks & PlayerInfoProt647::SpotAnim as u32 != 0 {
        renderer.write(buf, pid, PlayerInfoProt647::SpotAnim);
    }
    if masks & PlayerInfoProt647::Say as u32 != 0 {
        renderer.write(buf, pid, PlayerInfoProt647::Say);
    }
    if masks & PlayerInfoProt647::Appearance as u32 != 0 {
        renderer.write(buf, pid, PlayerInfoProt647::Appearance);
    }
}

/// Encodes the rev-647 NPC info packet for a single observer per tick.
pub struct NpcInfo {
    buf: Packet,
    updates: Packet,
    tracked: Vec<u16>,
    bw: BitWriter,
}

impl NpcInfo {
    const BITS_ADD: usize = 15 + 2 + 5 + 1 + 3 + 5 + 14 + 1;
    const BITS_CRAWL: usize = 1 + 2 + 1 + 3 + 1;
    const BITS_RUN: usize = 1 + 2 + 1 + 3 + 3 + 1;
    const BITS_WALK: usize = 1 + 2 + 3 + 1;
    const BITS_EXTEND: usize = 1 + 2;
    /// Per-payload cap. New spawns write an unaccounted low-def block
    /// (header + FaceCoord + any cached masks) into `updates`, so `fits`
    /// pads [`ADD_BLOCK_ESTIMATE`] per add; `buf` must additionally hold the
    /// spliced `updates` bytes, so it is allocated at twice this.
    const BYTES_LIMIT: usize = 20000;
    const ADD_BLOCK_ESTIMATE: usize = 24;

    #[inline]
    pub fn new() -> NpcInfo {
        NpcInfo {
            buf: Packet::new(Self::BYTES_LIMIT * 2),
            updates: Packet::new(Self::BYTES_LIMIT),
            tracked: Vec::with_capacity(BuildArea::PREFERRED_NPCS as usize),
            bw: BitWriter::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[inline]
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
    ) -> &[u8] {
        let build: &mut BuildArea = &mut active.player.build_area;

        if rebuild
            || dx > BuildArea::PREFERRED_VIEW_DISTANCE as i32
            || dz > BuildArea::PREFERRED_VIEW_DISTANCE as i32
        {
            build.rebuild_npcs(npcs);
        }

        self.updates.pos = 0;
        self.bw.reset();

        self.write_npcs(npcs, nsnap, renderer, active);
        self.write_new_npcs(map, npcs, nsnap, renderer, active);
        self.bw.pbit::<15>(&mut self.buf, 32767);
        self.bw.finish(&mut self.buf);
        if self.updates.pos > 0 {
            self.buf.pdata(&self.updates.data, 0, self.updates.pos);
        }
        unsafe { self.buf.data.get_unchecked(0..self.buf.pos) }
    }

    #[inline]
    fn write_npcs(
        &mut self,
        npcs: &mut [Option<Box<ActiveNpc>>],
        nsnap: &[NpcSnapshot],
        renderer: &mut NpcRenderer,
        active: &mut ActivePlayer,
    ) {
        // Swap the ids vec out of the build area — pointer swap, no copy.
        active.player.build_area.npcs.swap_ids(&mut self.tracked);
        self.bw.pbit::<8>(&mut self.buf, self.tracked.len() as i32);

        let obs_coord = active.player.pathing.coord;
        let obs_y = obs_coord.y();

        for i in 0..self.tracked.len() {
            let nid = self.tracked[i];
            let s = unsafe { *nsnap.get_unchecked(nid as usize) };
            if s.should_remove(obs_coord, obs_y, BuildArea::PREFERRED_VIEW_DISTANCE) {
                // Decrement observers on the live NPC (despawned slots skip it).
                if let Some(other) = npcs.get_mut(nid as usize).and_then(|n| n.as_mut()) {
                    other.npc.observers = other.npc.observers.saturating_sub(1);
                }
                self.remove(active, nid);
                continue;
            }

            let len: usize = s.len as usize;
            if s.run_dir != -1 {
                let crawl = npcs
                    .get(nid as usize)
                    .and_then(|n| n.as_ref())
                    .is_some_and(|n| n.npc.pathing.move_speed == rs_entity::MoveSpeed::Crawl);
                let extend = if crawl {
                    let extend = len > 0 && self.fits(NpcInfo::BITS_CRAWL, len);
                    self.crawl_dirs(s.walk_dir, s.run_dir, extend);
                    extend
                } else {
                    let extend = len > 0 && self.fits(NpcInfo::BITS_RUN, len);
                    self.run_dirs(s.walk_dir, s.run_dir, extend);
                    extend
                };
                if extend {
                    self.write_blocks_for(renderer, nid);
                }
            } else if s.walk_dir != -1 {
                let extend = len > 0 && self.fits(NpcInfo::BITS_WALK, len);
                self.bw.pbit::<1>(&mut self.buf, 1);
                self.bw.pbit::<2>(&mut self.buf, 1);
                self.bw
                    .pbit::<3>(&mut self.buf, NPC_DIR_MAP[s.walk_dir as usize]);
                self.bw.pbit::<1>(&mut self.buf, extend as i32);
                if extend {
                    self.write_blocks_for(renderer, nid);
                }
            } else if len > 0 && self.fits(NpcInfo::BITS_EXTEND, len) {
                self.bw.pbit::<1>(&mut self.buf, 1);
                self.bw.pbit::<2>(&mut self.buf, 0);
                self.write_blocks_for(renderer, nid);
            } else {
                self.bw.pbit::<1>(&mut self.buf, 0);
            }
        }
        active.player.build_area.npcs.swap_ids(&mut self.tracked);
        active.player.build_area.npcs.retain_bits();
    }

    #[inline]
    fn run_dirs(&mut self, walk_dir: i8, run_dir: i8, extend: bool) {
        self.bw.pbit::<1>(&mut self.buf, 1);
        self.bw.pbit::<2>(&mut self.buf, 2);
        self.bw.pbit::<1>(&mut self.buf, 1);
        self.bw
            .pbit::<3>(&mut self.buf, NPC_DIR_MAP[walk_dir as usize]);
        self.bw
            .pbit::<3>(&mut self.buf, NPC_DIR_MAP[run_dir as usize]);
        self.bw.pbit::<1>(&mut self.buf, extend as i32);
    }

    #[inline]
    fn crawl_dirs(&mut self, walk_dir: i8, _run_dir: i8, extend: bool) {
        self.bw.pbit::<1>(&mut self.buf, 1);
        self.bw.pbit::<2>(&mut self.buf, 2);
        self.bw.pbit::<1>(&mut self.buf, 0);
        self.bw
            .pbit::<3>(&mut self.buf, NPC_DIR_MAP[walk_dir as usize]);
        self.bw.pbit::<1>(&mut self.buf, extend as i32);
    }

    #[inline]
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
            if let Some(other) = npcs.get_mut(nid as usize).and_then(|n| n.as_mut()) {
                let len: usize = renderer.lowdefinitions(nid) + renderer.highdefinitions(nid);
                if !self.fits(NpcInfo::BITS_ADD, len + NpcInfo::ADD_BLOCK_ESTIMATE) {
                    return;
                }
                let x = other.npc.pathing.coord.x() as i32 - px as i32;
                let z = other.npc.pathing.coord.z() as i32 - pz as i32;
                self.bw
                    .pbit::<15>(&mut self.buf, other.npc.uid.nid() as i32);
                self.bw
                    .pbit::<2>(&mut self.buf, other.npc.pathing.coord.y() as i32);
                self.bw.pbit::<5>(&mut self.buf, x);
                self.bw.pbit::<1>(&mut self.buf, 1);
                // Its facing: an npc that cannot turn keeps it.
                let facing = other.npc.facing().map(|(fx, fz)| {
                    let size = other.npc.pathing.size;
                    (
                        CoordGrid::fine(other.npc.pathing.coord.x(), size) as i32 - fx as i32,
                        CoordGrid::fine(other.npc.pathing.coord.z(), size) as i32 - fz as i32,
                    )
                });
                self.bw.pbit::<3>(
                    &mut self.buf,
                    rs_protocol::network::game::server::npc_info::add_direction(facing) as i32,
                );
                self.bw.pbit::<5>(&mut self.buf, z);
                self.bw.pbit::<14>(&mut self.buf, other.npc.uid.id() as i32);
                self.bw.pbit::<1>(&mut self.buf, 0);
                self.lowdefinition(renderer, other);
                other.npc.observers = other.npc.observers.saturating_add(1);
                active.player.build_area.npcs.insert(other.npc.uid.nid());
            }
        }
    }

    #[inline]
    fn remove(&mut self, active: &mut ActivePlayer, other: u16) {
        // The ids vec is swapped out during `write_npcs`; clear the bit and
        // let the final `retain_bits` drop the id.
        self.bw.pbit::<1>(&mut self.buf, 1);
        self.bw.pbit::<2>(&mut self.buf, 3);
        active.player.build_area.npcs.remove_bit(other);
    }

    #[inline]
    fn write_blocks_for(&mut self, renderer: &mut NpcRenderer, nid: u16) {
        let mut masks: u32 = 0;
        for prot in [
            NpcInfoProt647::ChangeType,
            NpcInfoProt647::FaceCoord,
            NpcInfoProt647::SpotAnim,
            NpcInfoProt647::FaceEntity,
            NpcInfoProt647::Say,
            NpcInfoProt647::Hit,
            NpcInfoProt647::Anim,
        ] {
            if renderer.has(nid, prot) {
                masks |= prot as u32;
            }
        }
        self.write_blocks(renderer, nid, masks);
    }

    #[inline]
    fn lowdefinition(&mut self, renderer: &mut NpcRenderer, other: &ActiveNpc) {
        let nid: u16 = other.npc.uid.nid();
        let mut masks: u32 = 0;
        for prot in [
            NpcInfoProt647::ChangeType,
            NpcInfoProt647::SpotAnim,
            NpcInfoProt647::Say,
            NpcInfoProt647::Hit,
            NpcInfoProt647::Anim,
        ] {
            if renderer.has(nid, prot) {
                masks |= prot as u32;
            }
        }

        if let Some(face_entity) = other.npc.info.face_entity
            && !renderer.has(nid, NpcInfoProt647::FaceEntity)
        {
            renderer.cache_face_entity(nid, face_entity);
            masks |= NpcInfoProt647::FaceEntity as u32;
        } else if renderer.has(nid, NpcInfoProt647::FaceEntity) {
            masks |= NpcInfoProt647::FaceEntity as u32;
        }

        if !renderer.has(nid, NpcInfoProt647::FaceCoord) {
            if let (Some(face_x), Some(face_z)) = (other.npc.info.face_x, other.npc.info.face_z) {
                renderer.cache_face_coord(nid, face_x, face_z);
            } else if let (Some(orientation_x), Some(orientation_z)) =
                (other.npc.info.orientation_x, other.npc.info.orientation_z)
            {
                renderer.cache_face_coord(nid, orientation_x, orientation_z);
            } else {
                renderer.cache_face_coord(
                    nid,
                    CoordGrid::fine(other.npc.pathing.coord.x(), other.npc.pathing.size),
                    CoordGrid::fine(other.npc.pathing.coord.z(), other.npc.pathing.size),
                );
            }
        }

        masks |= NpcInfoProt647::FaceCoord as u32;

        self.write_blocks(renderer, nid, masks);
    }

    #[inline]
    fn write_blocks(&mut self, renderer: &mut NpcRenderer, nid: u16, mut masks: u32) {
        if (masks >> 16) != 0 {
            masks |= NPC_MASK_EXTENDED2;
        }
        if (masks >> 8) != 0 {
            masks |= NPC_MASK_EXTENDED;
        }
        self.updates.p1(masks as u8);
        if masks & NPC_MASK_EXTENDED != 0 {
            self.updates.p1((masks >> 8) as u8);
            if masks & NPC_MASK_EXTENDED2 != 0 {
                self.updates.p1((masks >> 16) as u8);
            }
        }
        // Client decode order (Statics1.method3939) for the implemented
        // subset: 0x2, 0x4, 0x20, ..., 0x10, ..., 0x40, ..., 0x1, ..., 0x80.
        for prot in [
            NpcInfoProt647::ChangeType,
            NpcInfoProt647::FaceCoord,
            NpcInfoProt647::SpotAnim,
            NpcInfoProt647::FaceEntity,
            NpcInfoProt647::Say,
            NpcInfoProt647::Hit,
            NpcInfoProt647::Anim,
        ] {
            if masks & prot as u32 != 0 {
                renderer.write(&mut self.updates, nid, prot);
            }
        }
    }

    #[inline]
    fn fits(&self, bits_to_add: usize, bytes_to_add: usize) -> bool {
        ((self.bw.bitpos() + bits_to_add + 7) >> 3) + self.updates.pos + bytes_to_add
            <= Self::BYTES_LIMIT - 3
    }
}

#[cfg(test)]
mod client_decode_tests {
    //! The mask blocks this module writes, read back by ports of the 647
    //! client's own mask loops: `Statics1.method3939` for npcs and
    //! `Class244.method1590` for players. Each block is read in the client's
    //! order with the client's byte transforms, so a block on the wrong bit,
    //! in the wrong place, or one byte off throws every block after it.

    use super::*;
    use rs_info::EntityMasks;
    use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};

    /// The deob-647 client `Packet` readers the mask loops use.
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
        fn g1_alt1(&mut self) -> i32 {
            -self.byte() & 0xFF
        }
        fn g1_alt2(&mut self) -> i32 {
            (128 - self.byte()) & 0xFF
        }
        fn g1_alt3(&mut self) -> i32 {
            (self.byte() - 128) & 0xFF
        }
        fn g2(&mut self) -> i32 {
            let hi = self.g1();
            (hi << 8) | self.g1()
        }
        fn ig2(&mut self) -> i32 {
            let lo = self.g1();
            lo | (self.g1() << 8)
        }
        fn g2_alt1(&mut self) -> i32 {
            let hi = self.g1();
            let lo = (self.byte() - 128) & 0xFF;
            (hi << 8) | lo
        }
        fn ig2_alt1(&mut self) -> i32 {
            let lo = (self.byte() - 128) & 0xFF;
            lo | (self.g1() << 8)
        }
        fn g4_alt1(&mut self) -> i32 {
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
        /// `count` hits: hitmark, damage, delay, health-bar fill.
        fn hits(&mut self, count: i32) -> Vec<(i32, i32, i32, i32)> {
            (0..count)
                .map(|_| (self.gsmart(), self.gsmart(), self.gsmart(), self.g1()))
                .collect()
        }
    }

    /// Two hits this tick, the second under the first, at the health they
    /// leave behind: 75 of 100, then 60 of 100.
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
        info.anim_id = Some(5577);
        info.anim_delay = Some(3);
        renderer.compute_info(9, &info);
        let mut npc_info = NpcInfo::new();
        npc_info.write_blocks_for(&mut renderer, 9);
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

        // In `method3939`'s order.
        assert_eq!(r.g2(), 4231, "0x2 change type");
        assert_eq!(
            (r.g2(), r.ig2()),
            (3222 * 2 + 1, 3218 * 2 + 1),
            "0x4 face coord, fine"
        );
        assert_eq!(r.ig2_alt1(), 74, "0x20 spotanim id");
        assert_eq!(r.g4_alt1(), 92 << 16 | 5, "0x20 spotanim height and delay");
        assert_eq!(r.g1_alt3() >> 3 & 0xF, 15, "0x20 no attachment");
        assert_eq!(r.ig2(), 0x8000 + 12, "0x10 face entity");
        assert_eq!(r.gjstr(), "Moo", "0x40 say");
        let count = r.g1_alt3();
        assert_eq!(
            r.hits(count),
            vec![(1, 25, 0, 75 * 255 / 100), (2, 150, 0, 60 * 255 / 100)],
            "0x1 both hits in one block"
        );
        for channel in 0..4 {
            assert_eq!(r.g2_alt1(), 5577, "0x80 anim channel {channel}");
        }
        assert_eq!(r.g1_alt1(), 3, "0x80 anim delay");
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
        let mut buf = Packet::new(256);
        let masks = PlayerInfo::collect_masks(&renderer, 7);
        write_mask_header(&mut buf, masks);
        write_mask_data(&mut buf, &mut renderer, 7, masks);
        let bytes = &buf.data[..buf.pos];

        let mut r = Reader { b: bytes, pos: 0 };
        let mut mask = r.g1();
        if mask & 0x20 != 0 {
            mask += r.g1() << 8;
        }
        if mask & 0x8000 != 0 {
            mask += r.g1() << 16;
        }
        assert_eq!(mask & !0x20, 0x80 | 0x2 | 0x10 | 0x40 | 0x200, "mask bits");

        // In `method1590`'s order.
        for channel in 0..4 {
            assert_eq!(r.g2(), 879, "0x80 anim channel {channel}");
        }
        assert_eq!(r.g1_alt3(), 3, "0x80 anim delay");
        let count = r.g1();
        assert_eq!(
            r.hits(count),
            vec![(1, 25, 0, 75 * 255 / 100), (2, 150, 0, 60 * 255 / 100)],
            "0x2 both hits in one block"
        );
        assert_eq!(r.ig2_alt1(), 42, "0x10 face entity");
        assert_eq!(r.g2_alt1(), 90, "0x40 spotanim id");
        assert_eq!(
            r.g4_alt1(),
            100 << 16 | 30,
            "0x40 spotanim height and delay"
        );
        assert_eq!(r.g1_alt2() >> 3 & 0xF, 15, "0x40 no attachment");
        assert_eq!(r.gjstr(), "Hi", "0x200 say");
        assert_eq!(r.pos, bytes.len(), "the client reads every byte written");
    }

    /// 2011 counts hits in life points, ten to a hitpoint, so a 30-hitpoint
    /// hit is 300 and passes a byte. The block's damage is a smart, and the
    /// bar's fill is content's (`ActivePlayer::hit`: current of 255).
    #[test]
    fn a_life_point_hit_past_255_reads_back_whole() {
        let mut renderer = PlayerRenderer::new();
        let mut info = EntityMasks::new();
        info.apply_damage(300, 3, 128, u8::MAX, PlayerInfoProt::Damage as u16);
        renderer.compute_info(7, &info, CoordGrid::new(3222, 0, 3218));
        let mut buf = Packet::new(64);
        let masks = PlayerInfo::collect_masks(&renderer, 7);
        write_mask_header(&mut buf, masks);
        write_mask_data(&mut buf, &mut renderer, 7, masks);
        let bytes = &buf.data[..buf.pos];

        let mut r = Reader { b: bytes, pos: 0 };
        assert_eq!(r.g1(), 0x2, "the hits mask alone");
        let count = r.g1();
        assert_eq!(r.hits(count), vec![(3, 300, 0, 128)]);
        assert_eq!(r.pos, bytes.len(), "the client reads every byte written");
    }
}
