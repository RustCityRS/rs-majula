use crate::active_npc::ActiveNpc;
use crate::active_player::ActivePlayer;
use crate::build::ActiveBuildArea;
use crate::engine::{OtherPlayers, engine};
use crate::watch::Target;
#[cfg(before_647)]
use rs_entity::MAX_PLAYERS;
use rs_entity::{BuildArea, MAX_NPCS};
#[cfg(since_647)]
use rs_entity::{dir_delta, face_dir};
use rs_grid::CoordGrid;
#[cfg(before_647)]
use rs_info::Visibility;
use rs_info::{NpcRenderer, PlayerRenderer};
use rs_io::Packet;
use rs_io::packet::BitWriter;
use rs_protocol::network::game::MAX_PACKET_SIZE;
use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};
#[cfg(since_647)]
use rs_protocol::network::game::server::npc_info::add_direction;
use rs_zone::zone_map::ZoneMap;

/// The most an info packet's body holds: its opcode and length take 3 bytes.
const BYTES_LIMIT: usize = MAX_PACKET_SIZE - 3;
#[cfg(before_647)]
const PLAYER_BUFFER_SIZE: usize = MAX_PACKET_SIZE;
#[cfg(since_647)]
const PLAYER_BUFFER_SIZE: usize = u16::MAX as usize;

/// Compact per-tick snapshot of the handful of player fields the hot
/// `write_players` loop reads, indexed by pid.
///
/// At 2000 players each observer's tracked loop visits up to ~250 entries, so
/// reading the movement decision straight out of a 12-byte struct keeps the
/// whole working set L1/L2-resident instead of chasing a random ~2.4 KB
/// `ActivePlayer` (3-4 cold cache lines) per entry. The full struct is touched
/// only for the minority of tracked players that actually carry a high-def
/// update block (`len > 0`).
///
/// Populated once per tick in the info phase (`phases/info.rs`), while every
/// live player slot is `Some`, and read-only thereafter during the output
/// phase. `flags` carries `PRESENT` (slot was live at snapshot time), `ACTIVE`,
/// `TELE`, and `VIS_HARD` so the removal predicate is byte-identical to reading
/// the live struct.
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

    /// The "no live player here" entry the snapshot is reset to each tick.
    pub const ABSENT: PlayerSnapshot = PlayerSnapshot {
        coord: 0,
        len: 0,
        run_dir: -1,
        walk_dir: -1,
        flags: 0,
    };

    /// Clears the `PRESENT` bit, mirroring a `players[pid].take()` / removal so
    /// later observers in the same tick encode a remove rather than movement.
    #[inline(always)]
    pub const fn clear(&mut self) {
        self.flags = 0;
    }

    /// Whether the tracked player should be removed from the observer's view
    /// this tick. Byte-for-byte the same condition the old `write_players`
    /// evaluated against the live `ActivePlayer` (`None` slot, teleport, level
    /// change, out of view distance, inactive, or hard-hidden).
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

/// Compact per-tick snapshot of the NPC fields the hot `write_npcs` loop reads,
/// indexed by nid. Mirrors [`PlayerSnapshot`] but without `VIS_HARD` (NPCs are not
/// visibility-filtered).
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
    #[cfg(since_647)]
    pub const CRAWL: u8 = 1 << 3;

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

#[cfg(since_647)]
const MAX_LOCAL_PLAYERS: usize = 255;

#[cfg(since_647)]
const WALK_DIR_MAP: [i32; 8] = [5, 6, 7, 3, 4, 0, 1, 2];
#[cfg(since_647)]
const NPC_DIR_MAP: [i32; 8] = [7, 0, 1, 6, 2, 5, 4, 3];

#[cfg(since_647)]
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
    // The pid of the player this packet is for: 2011 paints the hits they
    // took or dealt bright, everyone else's darker.
    #[cfg(since_647)]
    observer: u16,
}

impl PlayerInfo {
    #[cfg(before_647)]
    const BITS_ADD: usize = 11 + 5 + 5 + 1 + 1;
    #[cfg(before_647)]
    const BITS_RUN: usize = 1 + 2 + 3 + 3 + 1;
    #[cfg(before_647)]
    const BITS_WALK: usize = 1 + 2 + 3 + 1;
    #[cfg(before_647)]
    const BITS_EXTEND: usize = 1 + 2;
    #[cfg(before_647)]
    const BITS_END: usize = 11;

    /// Creates a new `PlayerInfo` encoder with pre-allocated buffers.
    ///
    /// # Returns
    /// A `PlayerInfo` with empty bit/byte buffers sized to
    /// [`MAX_PACKET_SIZE`] and a tracked-player vec
    /// pre-allocated to [`BuildArea::PREFERRED_PLAYERS`].
    #[inline(always)]
    pub fn new() -> PlayerInfo {
        PlayerInfo {
            buf: Packet::new(PLAYER_BUFFER_SIZE),
            updates: Packet::new(PLAYER_BUFFER_SIZE),
            #[cfg(before_647)]
            tracked: Vec::with_capacity(BuildArea::PREFERRED_PLAYERS as usize),
            bw: BitWriter::new(),
            #[cfg(since_647)]
            observer: 0,
        }
    }

    /// Encodes the full player info packet for a single tick.
    ///
    /// Rebuilds the build area if the observer has moved too far or a rebuild
    /// was requested, then writes the local player update, existing tracked
    /// players, and newly visible players into the bit-packed buffer. Update
    /// blocks are appended at the end.
    ///
    /// # Arguments
    /// * `renderer` - The player renderer that caches high/low definition data.
    /// * `players` - The full world player array.
    /// * `map` - The zone map for spatial lookups.
    /// * `active` - The observing player whose info packet is being built.
    /// * `dx` - Absolute X distance the observer moved this tick.
    /// * `dz` - Absolute Z distance the observer moved this tick.
    /// * `rebuild` - Whether a full build area rebuild is forced.
    ///
    /// # Returns
    /// The payload in two parts, the bit section and then the update blocks,
    /// valid until the next call to `encode`. They go out as one packet
    /// without being joined (see `ActivePlayer::player_info_parts`).
    ///
    /// # Call Stack
    /// **Calls:** [`write_local_player`](Self::write_local_player),
    /// [`write_players`](Self::write_players),
    /// [`write_new_players`](Self::write_new_players)
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn encode(
        &mut self,
        renderer: &mut PlayerRenderer,
        players: OtherPlayers<'_>,
        snap: &[PlayerSnapshot],
        map: &ZoneMap,
        active: &mut ActivePlayer,
        dx: i32,
        dz: i32,
        rebuild: bool,
    ) -> (&[u8], &[u8]) {
        #[cfg(since_647)]
        {
            self.observer = active.player.uid.pid();
        }
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
            let _ = (map, dx, dz, rebuild);
            active.player.build_area.resize();
        }

        self.updates.pos = 0;
        self.bw.reset();

        #[cfg(before_647)]
        {
            self.write_local_player(renderer, active);
            self.write_players(players, snap, renderer, active);
            self.write_new_players(map, players, snap, renderer, active);
            if self.updates.pos > 0 {
                self.bw
                    .pbit::<{ Self::BITS_END }>(&mut self.buf, MAX_PLAYERS as i32 - 1);
            }
        }
        #[cfg(since_647)]
        self.write_players(players, snap, renderer, active);
        self.bw.finish(&mut self.buf);
        #[cfg(before_647)]
        debug_assert!(self.buf.pos + self.updates.pos <= BYTES_LIMIT);
        unsafe {
            (
                self.buf.data.get_unchecked(..self.buf.pos),
                self.updates.data.get_unchecked(..self.updates.pos),
            )
        }
    }

    /// Writes the local (observing) player's movement and update block.
    ///
    /// Selects the appropriate movement encoding (teleport, run, walk, extend,
    /// or idle) based on the player's pathing state, and appends any pending
    /// high-definition update blocks.
    ///
    /// # Call Stack
    /// **Called by:** [`encode`](Self::encode)
    /// **Calls:** [`teleport`](Self::teleport), [`run`](Self::run),
    /// [`walk`](Self::walk), [`extend`](Self::extend), [`idle`](Self::idle)
    #[inline(always)]
    #[cfg(before_647)]
    fn write_local_player(&mut self, renderer: &mut PlayerRenderer, active: &ActivePlayer) {
        let len: usize = renderer.highdefinitions(active.player.uid.pid());
        if active.player.pathing.tele {
            self.teleport(
                renderer,
                active,
                active,
                active.player.pathing.coord.x() as i32
                    - active.player.build_area.origin.zone_origin_x() as i32,
                active.player.pathing.coord.y() as i32,
                active.player.pathing.coord.z() as i32
                    - active.player.build_area.origin.zone_origin_z() as i32,
                active.player.pathing.jump,
                len > 0,
            );
        } else if active.player.pathing.run_dir != -1 {
            self.run(renderer, active, active, len > 0);
        } else if active.player.pathing.walk_dir != -1 {
            self.walk(renderer, active, active, len > 0);
        } else if len > 0 {
            self.extend(renderer, active, active);
        } else {
            self.idle();
        }
    }

    /// Writes movement and update blocks for all previously tracked players.
    ///
    /// Iterates over the current tracked player set. Players that have
    /// teleported, changed level, moved out of view distance, gone inactive,
    /// or are hard-hidden are removed. Remaining players are encoded with
    /// their current movement mode (run/walk/extend/idle).
    ///
    /// # Call Stack
    /// **Called by:** [`encode`](Self::encode)
    /// **Calls:** [`remove`](Self::remove), [`run`](Self::run),
    /// [`walk`](Self::walk), [`extend`](Self::extend), [`idle`](Self::idle)
    #[inline(always)]
    #[cfg(before_647)]
    fn write_players(
        &mut self,
        players: OtherPlayers<'_>,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        active: &mut ActivePlayer,
    ) {
        // Swap the ids vec out of the build area -- pointer swap, no copy.
        // The bit vector stays in place so contains/remove_bit remain valid.
        active.player.build_area.players.swap_ids(&mut self.tracked);
        self.bw.pbit::<8>(&mut self.buf, self.tracked.len() as i32);

        // Hoist the observer's own coord / view distance out of the loop; only
        // the *other* coord is unpacked per iteration (was unpacked twice).
        let obs_coord = active.player.pathing.coord;
        let obs_y = obs_coord.y();
        let view_distance = active.player.build_area.view_distance;

        for i in 0..self.tracked.len() {
            let pid = self.tracked[i];
            // Read the movement decision from the compact snapshot, never the
            // ~2.4 KB ActivePlayer. The snapshot's PRESENT bit reproduces the
            // old `match players[pid] { None => remove }` (a logged-out or
            // emergency-removed player has its entry cleared). Every other field
            // is a verbatim copy of what the live struct held at info time, and
            // nothing mutates those fields between the info and output phases,
            // so the branch below is byte-identical to reading `players[pid]`.
            let s = unsafe { *snap.get_unchecked(pid as usize) };
            if s.should_remove(obs_coord, obs_y, view_distance) {
                self.remove(active, pid);
                continue;
            }

            let len: usize = s.len as usize;
            let later = (self.tracked.len() - i - 1) * PlayerInfo::BITS_RUN + PlayerInfo::BITS_END;
            if s.run_dir != -1 {
                // 1 + 2 + 3 + 3 + 1 = 10 bits (matches `run`)
                let extend = len > 0 && self.fits(PlayerInfo::BITS_RUN + later, len);
                self.bw.pbit::<10>(
                    &mut self.buf,
                    (((1 << 2) | 2) << 7)
                        | ((s.walk_dir as i32) << 4)
                        | ((s.run_dir as i32) << 1)
                        | (extend as i32),
                );
                if extend {
                    self.highdefinition_tracked(players, renderer, active, pid, s.flags);
                }
            } else if s.walk_dir != -1 {
                // 1 + 2 + 3 + 1 = 7 bits (matches `walk`)
                let extend = len > 0 && self.fits(PlayerInfo::BITS_WALK + later, len);
                self.bw.pbit::<7>(
                    &mut self.buf,
                    (((1 << 2) | 1) << 4) | ((s.walk_dir as i32) << 1) | (extend as i32),
                );
                if extend {
                    self.highdefinition_tracked(players, renderer, active, pid, s.flags);
                }
            } else if len > 0 && self.fits(PlayerInfo::BITS_EXTEND + later, len) {
                // 1 + 2 = 3 bits (matches `extend`)
                self.bw.pbit::<3>(&mut self.buf, 1 << 2);
                self.highdefinition_tracked(players, renderer, active, pid, s.flags);
            } else {
                self.bw.pbit::<1>(&mut self.buf, 0);
            }
        }
        // Swap the ids vec back and remove entries whose bits were cleared.
        active.player.build_area.players.swap_ids(&mut self.tracked);
        active.player.build_area.players.retain_bits();
    }

    #[cfg(since_647)]
    fn write_players(
        &mut self,
        players: OtherPlayers<'_>,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        active: &mut ActivePlayer,
    ) {
        let self_pid = active.player.uid.pid();
        let self_coord = active.player.pathing.coord;
        let view_dist = active.player.build_area.view_distance;
        let mut additions: usize = 0;

        self.write_local_players(
            snap, renderer, active, self_pid, self_coord, true, view_dist,
        );
        self.bw.finish(&mut self.buf);

        self.write_local_players(
            snap, renderer, active, self_pid, self_coord, false, view_dist,
        );
        self.bw.finish(&mut self.buf);

        self.write_external_players(
            players,
            snap,
            renderer,
            active,
            self_pid,
            self_coord,
            true,
            &mut additions,
            view_dist,
        );
        self.bw.finish(&mut self.buf);

        self.write_external_players(
            players,
            snap,
            renderer,
            active,
            self_pid,
            self_coord,
            false,
            &mut additions,
            view_dist,
        );
        self.bw.finish(&mut self.buf);

        active.player.build_area.cycle_nsn_flags();
    }

    /// Writes add-entries for newly visible players that are not yet tracked.
    ///
    /// Queries the build area for nearby players, skips those already tracked
    /// or hard-hidden, and encodes an add-entry for each until the buffer
    /// limit or the preferred player cap is reached.
    ///
    /// # Call Stack
    /// **Called by:** [`encode`](Self::encode)
    /// **Calls:** [`add`](Self::add)
    #[inline(always)]
    #[cfg(before_647)]
    fn write_new_players(
        &mut self,
        map: &ZoneMap,
        players: OtherPlayers<'_>,
        snap: &[PlayerSnapshot],
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
            if let Some(other) = players.get(pid) {
                if other.player.info.vis == Visibility::Hard {
                    continue;
                }
                let len: usize = renderer.lowdefinitions(pid) + renderer.highdefinitions(pid);
                if !self.fits(PlayerInfo::BITS_ADD + PlayerInfo::BITS_END, len) {
                    return;
                }
                self.add(
                    renderer,
                    active,
                    other,
                    other.player.uid.pid(),
                    other.player.pathing.coord.x() as i32 - active.player.pathing.coord.x() as i32,
                    other.player.pathing.coord.z() as i32 - active.player.pathing.coord.z() as i32,
                    other.player.pathing.jump,
                );
            }
        }
    }

    /// Encodes a player-add entry (23 bits) with position delta and jump flag,
    /// followed by the low-definition update block.
    ///
    /// # Arguments
    /// * `renderer` - The player renderer for cached appearance data.
    /// * `active` - The observing player.
    /// * `other` - The player being added.
    /// * `pid` - The player ID of the added player.
    /// * `x` - The X coordinate delta from the observer.
    /// * `z` - The Z coordinate delta from the observer.
    /// * `jump` - Whether the added player should be shown with a jump animation.
    ///
    /// # Side Effects
    /// * Inserts `pid` into the observer's tracked player set.
    ///
    /// # Call Stack
    /// **Called by:** [`write_new_players`](Self::write_new_players)
    /// **Calls:** [`lowdefinition`](Self::lowdefinition)
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    #[cfg(before_647)]
    fn add(
        &mut self,
        renderer: &mut PlayerRenderer,
        active: &mut ActivePlayer,
        other: &ActivePlayer,
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
        self.lowdefinition(renderer, active, other);
        active
            .player
            .build_area
            .players
            .insert(other.player.uid.pid());
    }

    /// Encodes a player-remove entry (3 bits) and clears the player's bit
    /// in the observer's tracked set.
    ///
    /// Only clears the bit (O(1)); the caller reconciles the ID list after
    /// the loop via [`IdBitSet::retain_bits`].
    ///
    /// # Arguments
    /// * `active` - The observing player whose build area is modified.
    /// * `other` - The player ID being removed.
    ///
    /// # Side Effects
    /// * Clears the bit for `other` in `active.player.build_area.players`.
    ///
    /// # Call Stack
    /// **Called by:** [`write_players`](Self::write_players)
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

    /// Encodes a teleport movement entry (21 bits) with absolute position
    /// relative to the build area origin, jump flag, and optional update block.
    ///
    /// # Arguments
    /// * `renderer` - The player renderer for update blocks.
    /// * `active` - The observing player.
    /// * `other` - The player being teleported.
    /// * `x` - The X coordinate relative to the build area origin.
    /// * `y` - The level (floor).
    /// * `z` - The Z coordinate relative to the build area origin.
    /// * `jump` - Whether a jump animation is shown.
    /// * `extend` - Whether a high-definition update block follows.
    ///
    /// # Call Stack
    /// **Called by:** [`write_local_player`](Self::write_local_player),
    /// [`write_players`](Self::write_players)
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    #[cfg(before_647)]
    fn teleport(
        &mut self,
        renderer: &mut PlayerRenderer,
        active: &ActivePlayer,
        other: &ActivePlayer,
        x: i32,
        y: i32,
        z: i32,
        jump: bool,
        extend: bool,
    ) {
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
        if extend {
            self.highdefinition(renderer, active, other);
        }
    }

    /// Encodes a run movement entry (10 bits) with walk and run directions,
    /// plus an optional high-definition update block.
    ///
    /// # Arguments
    /// * `renderer` - The player renderer for update blocks.
    /// * `active` - The observing player.
    /// * `other` - The player that ran.
    /// * `extend` - Whether a high-definition update block follows.
    ///
    /// # Call Stack
    /// **Called by:** [`write_local_player`](Self::write_local_player),
    /// [`write_players`](Self::write_players)
    #[inline(always)]
    #[cfg(before_647)]
    fn run(
        &mut self,
        renderer: &mut PlayerRenderer,
        active: &ActivePlayer,
        other: &ActivePlayer,
        extend: bool,
    ) {
        // 1 + 2 + 3 + 3 + 1 = 10 bits
        self.bw.pbit::<10>(
            &mut self.buf,
            (((1 << 2) | 2) << 7)
                | ((other.player.pathing.walk_dir as i32) << 4)
                | ((other.player.pathing.run_dir as i32) << 1)
                | (extend as i32),
        );
        if extend {
            self.highdefinition(renderer, active, other);
        }
    }

    /// Encodes a walk movement entry (7 bits) with walk direction, plus an
    /// optional high-definition update block.
    ///
    /// # Arguments
    /// * `renderer` - The player renderer for update blocks.
    /// * `active` - The observing player.
    /// * `other` - The player that walked.
    /// * `extend` - Whether a high-definition update block follows.
    ///
    /// # Call Stack
    /// **Called by:** [`write_local_player`](Self::write_local_player),
    /// [`write_players`](Self::write_players)
    #[inline(always)]
    #[cfg(before_647)]
    fn walk(
        &mut self,
        renderer: &mut PlayerRenderer,
        active: &ActivePlayer,
        other: &ActivePlayer,
        extend: bool,
    ) {
        // 1 + 2 + 3 + 1 = 7 bits
        self.bw.pbit::<7>(
            &mut self.buf,
            (((1 << 2) | 1) << 4) | ((other.player.pathing.walk_dir as i32) << 1) | (extend as i32),
        );
        if extend {
            self.highdefinition(renderer, active, other);
        }
    }

    /// Encodes a no-movement update (3 bits) that only carries a
    /// high-definition update block (e.g. appearance change while standing).
    ///
    /// # Call Stack
    /// **Called by:** [`write_local_player`](Self::write_local_player),
    /// [`write_players`](Self::write_players)
    #[inline(always)]
    #[cfg(before_647)]
    fn extend(
        &mut self,
        renderer: &mut PlayerRenderer,
        active: &ActivePlayer,
        other: &ActivePlayer,
    ) {
        // 1 + 2 = 3 bits
        self.bw.pbit::<3>(&mut self.buf, 1 << 2);
        self.highdefinition(renderer, active, other);
    }

    /// Encodes an idle entry (1 bit = 0), indicating no movement and no
    /// update blocks for this player.
    #[inline(always)]
    #[cfg(before_647)]
    fn idle(&mut self) {
        self.bw.pbit::<1>(&mut self.buf, 0);
    }

    /// Writes the high-definition update blocks for a player, masking out
    /// the Chat block when the observer is viewing themselves.
    ///
    /// # Call Stack
    /// **Calls:** [`write_blocks`](Self::write_blocks)
    #[inline(always)]
    #[cfg(before_647)]
    fn highdefinition(
        &mut self,
        renderer: &mut PlayerRenderer,
        active: &ActivePlayer,
        other: &ActivePlayer,
    ) {
        let pid = other.player.uid.pid();
        let masks = other.player.info.masks;
        if active.player.uid.pid() == pid {
            self.write_blocks(
                renderer,
                active,
                other,
                masks & !(PlayerInfoProt::Chat as u16),
            );
        } else if masks & PlayerInfoProt::ExactMove as u16 != 0 {
            self.write_blocks(renderer, active, other, masks);
        } else {
            let blk = renderer.high_block(pid);
            self.updates.pdata(blk, 0, blk.len());
        }
    }

    /// Emits the high-definition update for a tracked player, avoiding the cold
    /// ~2.4 KB `ActivePlayer` deref in the common case (B6). The coalesced high
    /// block is pid-addressed, so the live struct is only needed for
    /// self-observation (Chat is masked off, encoded field-by-field) or to
    /// append the observer-relative ExactMove tail (signalled by
    /// `PlayerSnapshot::HAS_EXACTMOVE`). Byte-identical to calling
    /// [`highdefinition`](Self::highdefinition) with the live struct: the else
    /// branch reproduces exactly the non-self, non-ExactMove path
    /// (`pdata(high_block(pid))` with no tail).
    #[inline(always)]
    #[cfg(before_647)]
    fn highdefinition_tracked(
        &mut self,
        players: OtherPlayers<'_>,
        renderer: &mut PlayerRenderer,
        active: &ActivePlayer,
        pid: u16,
        flags: u8,
    ) {
        if pid == active.player.uid.pid() || flags & PlayerSnapshot::HAS_EXACTMOVE != 0 {
            let other = unsafe { players.get(pid).unwrap_unchecked() };
            self.highdefinition(renderer, active, other);
        } else {
            let blk = renderer.high_block(pid);
            self.updates.pdata(blk, 0, blk.len());
        }
    }

    /// Writes the low-definition update blocks for a newly added player.
    ///
    /// Includes cached state that the observer has not yet seen, such as
    /// appearance (if changed since last observed), face entity, and face
    /// coordinate. Always includes the FaceCoord block.
    ///
    /// # Call Stack
    /// **Called by:** [`add`](Self::add)
    /// **Calls:** [`write_blocks`](Self::write_blocks)
    #[inline(always)]
    fn lowdefinition(
        &mut self,
        renderer: &mut PlayerRenderer,
        #[cfg_attr(since_647, allow(unused_variables))] active: &mut ActivePlayer,
        other: &ActivePlayer,
    ) {
        let pid = other.player.uid.pid();
        let mut masks = other.player.info.masks;

        #[cfg(before_647)]
        if let Some(last_appearance) = other.player.info.last_appearance
            && !active
                .player
                .build_area
                .has_appearance(pid, last_appearance)
        {
            active
                .player
                .build_area
                .save_appearance(pid, last_appearance);
            masks |= PlayerInfoProt::Appearance as u16;
        } else {
            masks &= !(PlayerInfoProt::Appearance as u16);
        }

        #[cfg(since_647)]
        if renderer.has(pid, PlayerInfoProt::Appearance) {
            masks |= PlayerInfoProt::Appearance as u16;
        }

        if let Some(face_entity) = other.player.info.face_entity {
            if !renderer.has(pid, PlayerInfoProt::FaceEntity) {
                renderer.cache_face_entity(pid, face_entity);
            }
            masks |= PlayerInfoProt::FaceEntity as u16;
        }

        if !renderer.has(pid, PlayerInfoProt::FaceCoord) {
            let (x, z) = other
                .player
                .info
                .face_x
                .zip(other.player.info.face_z)
                .or_else(|| {
                    other
                        .player
                        .info
                        .orientation_x
                        .zip(other.player.info.orientation_z)
                })
                .unwrap_or_else(|| {
                    (
                        CoordGrid::fine(other.player.pathing.coord.x(), 1),
                        CoordGrid::fine(other.player.pathing.coord.z(), 1),
                    )
                });
            #[cfg(before_647)]
            renderer.cache_face_coord(pid, x, z);
            #[cfg(since_647)]
            renderer.cache_face_coord(pid, x, z, other.player.pathing.coord);
        }

        masks |= PlayerInfoProt::FaceCoord as u16;

        #[cfg(before_647)]
        self.write_blocks(renderer, active, other, masks);
        #[cfg(since_647)]
        {
            renderer.cache_move_type(pid);
            masks |= PlayerInfoProt::MoveType as u16;
            self.write_blocks(renderer, pid, masks);
        }
    }

    /// Writes the individual update blocks (appearance, anim, face entity,
    /// say, damage, face coord, chat, spot anim, exact move) into the
    /// updates buffer based on the active mask bits.
    ///
    /// If the mask exceeds 0xFF, a two-byte extended mask header is written.
    ///
    /// # Arguments
    /// * `renderer` - The player renderer with cached block data.
    /// * `active` - The observing player (needed for ExactMove origin).
    /// * `other` - The player whose blocks are being written.
    /// * `masks` - The bitmask of active update blocks.
    ///
    /// # Call Stack
    /// **Called by:** [`highdefinition`](Self::highdefinition),
    /// [`lowdefinition`](Self::lowdefinition)
    #[inline(always)]
    #[cfg(before_647)]
    fn write_blocks(
        &mut self,
        renderer: &mut PlayerRenderer,
        active: &ActivePlayer,
        other: &ActivePlayer,
        masks: u16,
    ) {
        let pid = other.player.uid.pid();
        if masks > 0xff {
            self.updates.p2_alt1(masks | PlayerInfoProt::BigInfo as u16);
        } else {
            self.updates.p1(masks as u8);
        }
        if masks & PlayerInfoProt::Appearance as u16 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::Appearance);
        }
        if masks & PlayerInfoProt::Anim as u16 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::Anim);
        }
        if masks & PlayerInfoProt::FaceEntity as u16 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::FaceEntity);
        }
        if masks & PlayerInfoProt::Say as u16 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::Say);
        }
        if masks & PlayerInfoProt::Damage as u16 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::Damage);
        }
        if masks & PlayerInfoProt::FaceCoord as u16 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::FaceCoord);
        }
        if masks & PlayerInfoProt::Chat as u16 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::Chat);
        }
        if masks & PlayerInfoProt::SpotAnim as u16 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::SpotAnim);
        }
        if masks & PlayerInfoProt::ExactMove as u16 != 0 {
            let x = CoordGrid::zone_origin(active.player.build_area.origin.x());
            let z = CoordGrid::zone_origin(active.player.build_area.origin.z());
            renderer.write_exactmove(
                &mut self.updates,
                (other.player.info.exactmove_start_x.unwrap() - x) as u8,
                (other.player.info.exactmove_start_z.unwrap() - z) as u8,
                (other.player.info.exactmove_end_x.unwrap() - x) as u8,
                (other.player.info.exactmove_end_z.unwrap() - z) as u8,
                other.player.info.exactmove_begin.unwrap(),
                other.player.info.exactmove_finish.unwrap(),
                other.player.info.exactmove_dir.unwrap(),
            )
        }
        #[cfg(since_244)]
        if masks & PlayerInfoProt::Damage2 as u16 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::Damage2);
        }
    }

    #[cfg(since_647)]
    fn write_blocks(&mut self, renderer: &PlayerRenderer, pid: u16, masks: u16) {
        let wire = PlayerInfoProt::mask(masks);
        let (header, len) = PlayerInfoProt::header(wire);
        self.updates.pdata(&header, 0, len);
        if wire & PlayerInfoProt::FaceCoord as u32 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::FaceCoord);
        }
        if wire & PlayerInfoProt::TempMoveType as u32 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::TempMoveType);
        }
        if wire & PlayerInfoProt::Anim as u32 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::Anim);
        }
        if wire & PlayerInfoProt::MoveType as u32 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::MoveType);
        }
        if wire & PlayerInfoProt::ExactMove as u32 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::ExactMove);
        }
        if wire & PlayerInfoProt::Damage as u32 != 0 {
            renderer.write_hits_for(&mut self.updates, pid, self.observer);
        }
        if wire & PlayerInfoProt::FaceEntity as u32 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::FaceEntity);
        }
        if wire & PlayerInfoProt::SpotAnim as u32 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::SpotAnim);
        }
        if wire & PlayerInfoProt::Say as u32 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::Say);
        }
        if wire & PlayerInfoProt::Appearance as u32 != 0 {
            renderer.write(&mut self.updates, pid, PlayerInfoProt::Appearance);
        }
    }

    /// Checks whether the buffer has room for the given number of additional
    /// bits and bytes without exceeding the packet size limit.
    ///
    /// # Arguments
    /// * `bits_to_add` - Number of bits the next movement entry requires.
    /// * `bytes_to_add` - Number of bytes the next update block requires.
    ///
    /// # Returns
    /// `true` if the combined bit buffer (rounded up to bytes) plus the
    /// update buffer plus the new data fits within [`BYTES_LIMIT`].
    #[inline(always)]
    #[cfg(before_647)]
    fn fits(&self, bits_to_add: usize, bytes_to_add: usize) -> bool {
        ((self.bw.bitpos() + bits_to_add + 7) >> 3) + self.updates.pos + bytes_to_add <= BYTES_LIMIT
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
                // 1 + 2 + 2 + 7 + 7 + 1 + 1 = 21 bits: teleport, jump, update.
                info.bw.pbit::<21>(
                    &mut info.buf,
                    (((1 << 2) | 3) << 18)
                        | ((coord.y() as i32 & 0x3) << 16)
                        | (((coord.x() as i32 - build.origin.zone_origin_x() as i32) & 0x7F) << 9)
                        | (((coord.z() as i32 - build.origin.zone_origin_z() as i32) & 0x7F) << 2)
                        | (1 << 1)
                        | 1,
                );
                let masks = Self::keyframe_masks(renderer, active);
                info.write_blocks(renderer, active, active, masks);
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
                    Some(other) => {
                        let masks = Self::keyframe_masks(renderer, other);
                        info.write_blocks(renderer, active, other, masks);
                    }
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

    /// What a keyframe shows of `other`: its appearance and which way it
    /// faces, as a new observer's add does.
    #[cfg(before_647)]
    fn keyframe_masks(renderer: &mut PlayerRenderer, other: &ActivePlayer) -> u16 {
        let pid = other.player.uid.pid();
        let mut masks = 0;
        if renderer.has(pid, PlayerInfoProt::Appearance) {
            masks |= PlayerInfoProt::Appearance as u16;
        }
        if let Some(face_entity) = other.player.info.face_entity {
            if !renderer.has(pid, PlayerInfoProt::FaceEntity) {
                renderer.cache_face_entity(pid, face_entity);
            }
            masks |= PlayerInfoProt::FaceEntity as u16;
        }
        if !renderer.has(pid, PlayerInfoProt::FaceCoord) {
            if let (Some(x), Some(z)) = (other.player.info.face_x, other.player.info.face_z) {
                renderer.cache_face_coord(pid, x, z);
            } else if let (Some(x), Some(z)) = (
                other.player.info.orientation_x,
                other.player.info.orientation_z,
            ) {
                renderer.cache_face_coord(pid, x, z);
            } else {
                renderer.cache_face_coord(
                    pid,
                    CoordGrid::fine(other.player.pathing.coord.x(), 1),
                    CoordGrid::fine(other.player.pathing.coord.z(), 1),
                );
            }
        }
        masks | PlayerInfoProt::FaceCoord as u16
    }

    #[cfg(since_647)]
    #[allow(clippy::too_many_arguments)]
    fn write_local_players(
        &mut self,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        self_pid: u16,
        self_coord: CoordGrid,
        nsn: bool,
        view_dist: u8,
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
                    let removing = !CoordGrid::from(s.coord).in_distance(self_coord, view_dist)
                        || CoordGrid::from(s.coord).y() != self_coord.y()
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
            self.write_player(snap, renderer, observer, index, is_self, removing);
        }

        if skip >= 0 {
            self.bw.pbit::<1>(&mut self.buf, 0);
            encode_skip(&mut self.bw, &mut self.buf, skip as usize);
        }
    }

    #[cfg(since_647)]
    fn write_player(
        &mut self,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        index: usize,
        is_self: bool,
        removing: bool,
    ) {
        if removing {
            self.bw.pbit::<1>(&mut self.buf, 0);
            self.bw.pbit::<2>(&mut self.buf, 0);
            self.validate_coordinates(snap, observer, index);
            observer.player.build_area.players.remove(index as u16);
            if observer.watch.checking() {
                observer
                    .watch
                    .anticheat
                    .lost(Target::Player(index as u16), engine().clock);
            }
            return;
        }

        let (tele, walk_dir, run_dir, coord) = if is_self {
            (
                observer.player.pathing.tele,
                observer.player.pathing.walk_dir,
                observer.player.pathing.run_dir,
                observer.player.pathing.coord,
            )
        } else {
            let s = unsafe { *snap.get_unchecked(index) };
            (
                s.flags & PlayerSnapshot::TELE != 0,
                s.walk_dir,
                s.run_dir,
                CoordGrid::from(s.coord),
            )
        };

        let has_blocks = renderer.highdefinitions(index as u16) > 0;

        if tele {
            let last = observer.player.build_area.player_last_coords[index];
            let dx = coord.x() as i32 - last.x() as i32;
            let dz = coord.z() as i32 - last.z() as i32;
            let dl = coord.y() as i32 - last.y() as i32;
            let local = dx.abs() <= 15 && dz.abs() <= 15;

            self.bw.pbit::<1>(&mut self.buf, has_blocks as i32);
            self.bw.pbit::<2>(&mut self.buf, 3);
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
                self.highdefinition(renderer, index as u16);
            }
            observer.player.build_area.player_last_coords[index] = coord;
        } else if run_dir != -1 {
            let (wx, wz) = dir_delta(walk_dir);
            let (rx, rz) = dir_delta(run_dir);
            let dx = wx as i32 + rx as i32;
            let dz = wz as i32 + rz as i32;
            self.bw.pbit::<1>(&mut self.buf, has_blocks as i32);
            if let Some(ri) = run_index(dx, dz) {
                self.bw.pbit::<2>(&mut self.buf, 2);
                self.bw.pbit::<4>(&mut self.buf, ri);
            } else {
                self.bw.pbit::<2>(&mut self.buf, 1);
                self.bw.pbit::<3>(
                    &mut self.buf,
                    WALK_DIR_MAP[face_dir(0, 0, dx, dz).max(0) as usize],
                );
            }
            if has_blocks {
                self.highdefinition(renderer, index as u16);
            }
            observer.player.build_area.player_last_coords[index] = coord;
        } else if walk_dir != -1 {
            self.bw.pbit::<1>(&mut self.buf, has_blocks as i32);
            self.bw.pbit::<2>(&mut self.buf, 1);
            self.bw
                .pbit::<3>(&mut self.buf, WALK_DIR_MAP[walk_dir as usize]);
            if has_blocks {
                self.highdefinition(renderer, index as u16);
            }
            observer.player.build_area.player_last_coords[index] = coord;
        } else if has_blocks {
            self.bw.pbit::<1>(&mut self.buf, 1);
            self.bw.pbit::<2>(&mut self.buf, 0);
            self.highdefinition(renderer, index as u16);
        }
    }

    #[cfg(since_647)]
    #[allow(clippy::too_many_arguments)]
    fn write_external_players(
        &mut self,
        players: OtherPlayers<'_>,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        self_pid: u16,
        self_coord: CoordGrid,
        nsn: bool,
        additions: &mut usize,
        view_dist: u8,
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
                && s.flags & PlayerSnapshot::VIS_HARD == 0
                && CoordGrid::from(s.coord).in_distance(self_coord, view_dist)
                && CoordGrid::from(s.coord).y() == self_coord.y();

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
            self.add_player(players, snap, renderer, observer, index);
            *additions += 1;
        }

        if skip >= 0 {
            self.bw.pbit::<1>(&mut self.buf, 0);
            encode_skip(&mut self.bw, &mut self.buf, skip as usize);
        }
    }

    #[cfg(since_647)]
    fn add_player(
        &mut self,
        players: OtherPlayers<'_>,
        snap: &[PlayerSnapshot],
        renderer: &mut PlayerRenderer,
        observer: &mut ActivePlayer,
        index: usize,
    ) {
        let coord = CoordGrid::from(unsafe { *snap.get_unchecked(index) }.coord);

        self.bw.pbit::<2>(&mut self.buf, 0);
        self.validate_coordinates(snap, observer, index);
        self.bw.pbit::<6>(&mut self.buf, (coord.x() & 0x3F) as i32);
        self.bw.pbit::<6>(&mut self.buf, (coord.z() & 0x3F) as i32);
        self.bw.pbit::<1>(&mut self.buf, 1);

        let other = unsafe { players.get(index as u16).unwrap_unchecked() };
        self.lowdefinition(renderer, observer, other);

        observer.player.build_area.players.insert(index as u16);
        observer.player.build_area.player_last_coords[index] = coord;
        observer.player.build_area.nsn_flags[index] |= 2;
    }

    #[cfg(since_647)]
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

    #[cfg(since_647)]
    fn highdefinition(&mut self, renderer: &mut PlayerRenderer, pid: u16) {
        renderer.write_high_block_for(&mut self.updates, pid, self.observer);
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
    // The pid of the player this packet is for: 2011 paints the hits they
    // dealt bright, everyone else's darker.
    #[cfg(since_647)]
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
    const BITS_CRAWL: usize = 1 + 2 + 1 + 3 + 1;
    #[cfg(since_647)]
    const BITS_ADD: usize = 15 + 2 + 5 + 1 + 3 + 5 + 14 + 1;
    const BITS_WALK: usize = 1 + 2 + 3 + 1;
    const BITS_EXTEND: usize = 1 + 2;
    #[cfg(before_254)]
    const BITS_END: usize = 13;
    #[cfg(all(since_254, before_647))]
    const BITS_END: usize = 14;
    #[cfg(since_647)]
    const BITS_END: usize = 15;

    /// Creates a new `NpcInfo` encoder with pre-allocated buffers.
    ///
    /// # Returns
    /// An `NpcInfo` with empty bit/byte buffers sized to
    /// [`MAX_PACKET_SIZE`] and a tracked-NPC vec
    /// pre-allocated to [`BuildArea::PREFERRED_NPCS`].
    #[inline(always)]
    pub fn new() -> NpcInfo {
        NpcInfo {
            buf: Packet::new(MAX_PACKET_SIZE),
            updates: Packet::new(MAX_PACKET_SIZE),
            tracked: Vec::with_capacity(BuildArea::PREFERRED_NPCS as usize),
            bw: BitWriter::new(),
            #[cfg(since_647)]
            observer: 0,
        }
    }

    /// Encodes the full NPC info packet for a single tick.
    ///
    /// Rebuilds the NPC build area if the observer moved too far or a rebuild
    /// was forced, then writes existing tracked NPCs and newly visible NPCs
    /// into the bit-packed buffer. Update blocks are appended at the end.
    ///
    /// # Arguments
    /// * `renderer` - The NPC renderer that caches high/low definition data.
    /// * `npcs` - The full world NPC array (mutable for observer counting).
    /// * `map` - The zone map for spatial lookups.
    /// * `active` - The observing player whose NPC info packet is being built.
    /// * `dx` - Absolute X distance the observer moved this tick.
    /// * `dz` - Absolute Z distance the observer moved this tick.
    /// * `rebuild` - Whether a full NPC build area rebuild is forced.
    ///
    /// # Returns
    /// The payload in two parts, the bit section and then the update blocks,
    /// valid until the next call to `encode`. They go out as one packet
    /// without being joined (see `ActivePlayer::npc_info_parts`).
    ///
    /// # Call Stack
    /// **Calls:** [`write_npcs`](Self::write_npcs),
    /// [`write_new_npcs`](Self::write_new_npcs)
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
        #[cfg(since_647)]
        {
            self.observer = active.player.uid.pid();
        }
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
    ///
    /// # Side Effects
    /// * Decrements `npc.observers` for removed NPCs.
    ///
    /// # Call Stack
    /// **Called by:** [`encode`](Self::encode)
    #[inline(always)]
    fn write_npcs(
        &mut self,
        npcs: &mut [Option<Box<ActiveNpc>>],
        nsnap: &[NpcSnapshot],
        renderer: &mut NpcRenderer,
        active: &mut ActivePlayer,
    ) {
        // Swap the ids vec out of the build area -- pointer swap, no copy.
        // The bit vector stays in place so contains/remove_bit remain valid.
        active.player.build_area.npcs.swap_ids(&mut self.tracked);
        self.bw.pbit::<8>(&mut self.buf, self.tracked.len() as i32);

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
                self.remove(active, nid);
                continue;
            }

            let len: usize = s.len as usize;
            let later = (self.tracked.len() - i - 1) * NpcInfo::BITS_RUN + NpcInfo::BITS_END;
            let running = s.run_dir != -1;
            #[cfg(since_647)]
            let crawl = s.flags & NpcSnapshot::CRAWL != 0;
            #[cfg(since_647)]
            let running = running || (crawl && s.walk_dir != -1);
            if running {
                // 1 + 2 + 3 + 3 + 1 = 10 bits
                #[cfg(before_647)]
                let extend = {
                    let extend = len > 0 && self.fits(NpcInfo::BITS_RUN + later, len);
                    self.bw.pbit::<10>(
                        &mut self.buf,
                        (((1 << 2) | 2) << 7)
                            | ((s.walk_dir as i32) << 4)
                            | ((s.run_dir as i32) << 1)
                            | (extend as i32),
                    );
                    extend
                };
                #[cfg(since_647)]
                let extend = {
                    let bits = if crawl {
                        Self::BITS_CRAWL
                    } else {
                        Self::BITS_RUN
                    };
                    let extend = len > 0 && self.fits(bits + later, len);
                    self.bw
                        .pbit::<4>(&mut self.buf, (1 << 3) | (2 << 1) | !crawl as i32);
                    self.bw
                        .pbit::<3>(&mut self.buf, NPC_DIR_MAP[s.walk_dir as usize]);
                    if !crawl {
                        self.bw
                            .pbit::<3>(&mut self.buf, NPC_DIR_MAP[s.run_dir as usize]);
                    }
                    self.bw.pbit::<1>(&mut self.buf, extend as i32);
                    extend
                };
                if extend {
                    self.highdefinition(renderer, nid);
                }
            } else if s.walk_dir != -1 {
                // 1 + 2 + 3 + 1 = 7 bits
                let extend = len > 0 && self.fits(NpcInfo::BITS_WALK + later, len);
                #[cfg(before_647)]
                self.bw.pbit::<7>(
                    &mut self.buf,
                    (((1 << 2) | 1) << 4) | ((s.walk_dir as i32) << 1) | (extend as i32),
                );
                #[cfg(since_647)]
                self.bw.pbit::<7>(
                    &mut self.buf,
                    (((1 << 2) | 1) << 4) | (NPC_DIR_MAP[s.walk_dir as usize] << 1) | extend as i32,
                );
                if extend {
                    self.highdefinition(renderer, nid);
                }
            } else if len > 0 && self.fits(NpcInfo::BITS_EXTEND + later, len) {
                // 1 + 2 = 3 bits
                self.bw.pbit::<3>(&mut self.buf, 1 << 2);
                self.highdefinition(renderer, nid);
            } else {
                self.bw.pbit::<1>(&mut self.buf, 0);
            }
        }
        // Swap the ids vec back and remove entries whose bits were cleared.
        active.player.build_area.npcs.swap_ids(&mut self.tracked);
        active.player.build_area.npcs.retain_bits();
    }

    /// Writes add-entries for newly visible NPCs that are not yet tracked.
    ///
    /// Queries the build area for nearby NPCs, skips those already tracked,
    /// and encodes an add-entry for each until the buffer limit or the
    /// preferred NPC cap is reached. Increments the observer count on each
    /// newly tracked NPC.
    ///
    /// # Side Effects
    /// * Increments `npc.observers` for added NPCs.
    ///
    /// # Call Stack
    /// **Called by:** [`encode`](Self::encode)
    /// **Calls:** [`add`](Self::add)
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
                self.add(
                    renderer,
                    active,
                    other,
                    other.npc.uid.nid(),
                    other.npc.uid.id(),
                    other.npc.pathing.coord.x() as i32 - active.player.pathing.coord.x() as i32,
                    other.npc.pathing.coord.z() as i32 - active.player.pathing.coord.z() as i32,
                    other.npc.pathing.jump,
                );
                other.npc.observers = other.npc.observers.saturating_add(1);
            }
        }
    }

    /// Encodes an NPC-add entry (35 bits split across two pbit calls) with
    /// the NPC ID, type, position delta, and low-definition update block.
    ///
    /// # Arguments
    /// * `renderer` - The NPC renderer for cached block data.
    /// * `active` - The observing player.
    /// * `other` - The NPC being added.
    /// * `nid` - The NPC slot index.
    /// * `ntype` - The NPC type ID (may differ from base type if morphed).
    /// * `x` - The X coordinate delta from the observer.
    /// * `z` - The Z coordinate delta from the observer.
    /// * `jump` - Whether the NPC teleported; encoded in the position bits since rev 274.
    ///
    /// # Side Effects
    /// * Inserts `nid` into the observer's tracked NPC set.
    ///
    /// # Call Stack
    /// **Called by:** [`write_new_npcs`](Self::write_new_npcs)
    /// **Calls:** [`lowdefinition`](Self::lowdefinition)
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    fn add(
        &mut self,
        renderer: &mut NpcRenderer,
        active: &mut ActivePlayer,
        other: &ActiveNpc,
        nid: u16,
        ntype: u16,
        x: i32,
        z: i32,
        #[cfg_attr(any(before_274, since_647), allow(unused_variables))] jump: bool,
    ) {
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
        self.bw.pbit::<12>(
            &mut self.buf,
            ((x & 0x1F) << 7) | ((z & 0x1F) << 2) | ((jump as i32) << 1) | 1,
        );
        #[cfg(since_647)]
        {
            self.bw.pbit::<15>(&mut self.buf, nid as i32);
            self.bw
                .pbit::<2>(&mut self.buf, other.npc.pathing.coord.y() as i32);
            self.bw.pbit::<5>(&mut self.buf, x);
            self.bw.pbit::<1>(&mut self.buf, 1);
            let facing = other.npc.facing().map(|(fx, fz)| {
                let size = other.npc.pathing.size;
                (
                    CoordGrid::fine(other.npc.pathing.coord.x(), size) as i32 - fx as i32,
                    CoordGrid::fine(other.npc.pathing.coord.z(), size) as i32 - fz as i32,
                )
            });
            self.bw
                .pbit::<3>(&mut self.buf, add_direction(facing) as i32);
            self.bw.pbit::<5>(&mut self.buf, z);
            self.bw.pbit::<14>(&mut self.buf, ntype as i32);
            self.bw.pbit::<1>(&mut self.buf, 0);
        }
        self.lowdefinition(renderer, other);
        active.player.build_area.npcs.insert(other.npc.uid.nid());
    }

    /// Encodes an NPC-remove entry (3 bits) and clears the NPC's bit in
    /// the observer's tracked set.
    ///
    /// Only clears the bit (O(1)); the caller reconciles the ID list after
    /// the loop via [`IdBitSet::retain_bits`].
    ///
    /// # Arguments
    /// * `active` - The observing player whose build area is modified.
    /// * `other` - The NPC slot index being removed.
    ///
    /// # Side Effects
    /// * Clears the bit for `other` in `active.player.build_area.npcs`.
    ///
    /// # Call Stack
    /// **Called by:** [`write_npcs`](Self::write_npcs)
    #[inline(always)]
    fn remove(&mut self, active: &mut ActivePlayer, other: u16) {
        // 1 + 2 = 3 bits
        self.bw.pbit::<3>(&mut self.buf, (1 << 2) | 3);
        active.player.build_area.npcs.remove_bit(other);
        if active.watch.checking() {
            active
                .watch
                .anticheat
                .lost(Target::Npc(other), engine().clock);
        }
    }

    /// Writes the pre-coalesced high-definition update block for an NPC with a
    /// single `pdata`.
    ///
    /// NPC high-def blocks are observer-independent, so the entire block (mask
    /// header + every field) was pre-built in the producer
    /// ([`NpcRenderer::compute_info`]); the encoder needs only `nid`. (The
    /// low-def add path still uses the field-by-field `write_blocks`, since its
    /// mask is recomputed per observer.)
    ///
    /// # Call Stack
    /// **Called by:** [`write_npcs`](Self::write_npcs)
    #[inline(always)]
    fn highdefinition(&mut self, renderer: &NpcRenderer, nid: u16) {
        #[cfg(before_647)]
        {
            let blk = renderer.high_block(nid);
            self.updates.pdata(blk, 0, blk.len());
        }
        #[cfg(since_647)]
        renderer.write_high_block_for(&mut self.updates, nid, self.observer);
    }

    /// Writes the low-definition update blocks for a newly added NPC.
    ///
    /// Includes cached state that the observer has not yet seen: face entity
    /// and face coordinate. Always includes the FaceCoord block.
    ///
    /// # Call Stack
    /// **Called by:** [`add`](Self::add)
    /// **Calls:** [`write_blocks`](Self::write_blocks)
    #[inline(always)]
    fn lowdefinition(&mut self, renderer: &mut NpcRenderer, other: &ActiveNpc) {
        let nid = other.npc.uid.nid();
        let mut masks = other.npc.info.masks;

        if let Some(face_entity) = other.npc.info.face_entity {
            if !renderer.has(nid, NpcInfoProt::FaceEntity) {
                renderer.cache_face_entity(nid, face_entity);
            }
            masks |= NpcInfoProt::FaceEntity as u16;
        }

        if !renderer.has(nid, NpcInfoProt::FaceCoord) {
            if let (Some(face_x), Some(face_z)) = (other.npc.info.face_x, other.npc.info.face_z) {
                renderer.cache_face_coord(nid, face_x, face_z);
            } else if let (Some(orientation_x), Some(orientation_z)) =
                (other.npc.info.orientation_x, other.npc.info.orientation_z)
            {
                renderer.cache_face_coord(nid, orientation_x, orientation_z);
            } else {
                #[cfg(before_647)]
                let size = 1;
                #[cfg(since_647)]
                let size = other.npc.pathing.size;
                renderer.cache_face_coord(
                    nid,
                    CoordGrid::fine(other.npc.pathing.coord.x(), size),
                    CoordGrid::fine(other.npc.pathing.coord.z(), size),
                );
            }
        }

        masks |= NpcInfoProt::FaceCoord as u16;

        self.write_blocks(renderer, nid, masks);
    }

    /// Writes the individual NPC update blocks (anim, face entity, say,
    /// damage, change type, spot anim, face coord) into the updates buffer
    /// based on the active mask bits.
    ///
    /// # Arguments
    /// * `renderer` - The NPC renderer with cached block data.
    /// * `nid` - The NPC slot index whose blocks are being written.
    /// * `masks` - The bitmask of active update blocks.
    ///
    /// # Call Stack
    /// **Called by:** [`highdefinition`](Self::highdefinition),
    /// [`lowdefinition`](Self::lowdefinition)
    #[inline(always)]
    #[cfg(before_647)]
    fn write_blocks(&mut self, renderer: &mut NpcRenderer, nid: u16, masks: u16) {
        self.updates.p1(masks as u8);
        // ----
        // an optimization *could* be made where all of these are just 1 block of bytes...
        // the same could NOT be done for players bcuz of how exact_move works...
        #[cfg(since_244)]
        if masks & NpcInfoProt::Damage2 as u16 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::Damage2);
        }
        if masks & NpcInfoProt::Anim as u16 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::Anim);
        }
        if masks & NpcInfoProt::FaceEntity as u16 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::FaceEntity);
        }
        if masks & NpcInfoProt::Say as u16 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::Say);
        }
        if masks & NpcInfoProt::Damage as u16 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::Damage);
        }
        if masks & NpcInfoProt::ChangeType as u16 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::ChangeType);
        }
        if masks & NpcInfoProt::SpotAnim as u16 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::SpotAnim);
        }
        if masks & NpcInfoProt::FaceCoord as u16 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::FaceCoord);
        }
    }

    #[cfg(since_647)]
    fn write_blocks(&mut self, renderer: &NpcRenderer, nid: u16, masks: u16) {
        let wire = NpcInfoProt::mask(masks);
        let (header, len) = NpcInfoProt::header(wire);
        self.updates.pdata(&header, 0, len);
        if wire & NpcInfoProt::ChangeType as u32 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::ChangeType);
        }
        if wire & NpcInfoProt::FaceCoord as u32 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::FaceCoord);
        }
        if wire & NpcInfoProt::SpotAnim as u32 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::SpotAnim);
        }
        if wire & NpcInfoProt::FaceEntity as u32 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::FaceEntity);
        }
        if wire & NpcInfoProt::Say as u32 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::Say);
        }
        if wire & NpcInfoProt::Damage as u32 != 0 {
            renderer.write_hits_for(&mut self.updates, nid, self.observer);
        }
        if wire & NpcInfoProt::Anim as u32 != 0 {
            renderer.write(&mut self.updates, nid, NpcInfoProt::Anim);
        }
    }

    /// Checks whether the buffer has room for the given number of additional
    /// bits and bytes without exceeding the NPC info packet size limit.
    ///
    /// # Arguments
    /// * `bits_to_add` - Number of bits the next movement entry requires.
    /// * `bytes_to_add` - Number of bytes the next update block requires.
    ///
    /// # Returns
    /// `true` if the combined bit buffer (rounded up to bytes) plus the
    /// update buffer plus the new data fits within [`BYTES_LIMIT`].
    #[inline(always)]
    fn fits(&self, bits_to_add: usize, bytes_to_add: usize) -> bool {
        ((self.bw.bitpos() + bits_to_add + 7) >> 3) + self.updates.pos + bytes_to_add <= BYTES_LIMIT
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
                    Some(other) => {
                        let masks = Self::keyframe_masks(renderer, other);
                        info.write_blocks(renderer, nid, masks);
                    }
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

    /// What a keyframe shows of `other`: which way it faces, as a new
    /// observer's add does.
    #[cfg(before_647)]
    fn keyframe_masks(renderer: &mut NpcRenderer, other: &ActiveNpc) -> u16 {
        let nid = other.npc.uid.nid();
        let mut masks = 0;
        if let Some(face_entity) = other.npc.info.face_entity {
            if !renderer.has(nid, NpcInfoProt::FaceEntity) {
                renderer.cache_face_entity(nid, face_entity);
            }
            masks |= NpcInfoProt::FaceEntity as u16;
        }
        if !renderer.has(nid, NpcInfoProt::FaceCoord) {
            if let (Some(x), Some(z)) = (other.npc.info.face_x, other.npc.info.face_z) {
                renderer.cache_face_coord(nid, x, z);
            } else if let (Some(x), Some(z)) =
                (other.npc.info.orientation_x, other.npc.info.orientation_z)
            {
                renderer.cache_face_coord(nid, x, z);
            } else {
                renderer.cache_face_coord(
                    nid,
                    CoordGrid::fine(other.npc.pathing.coord.x(), 1),
                    CoordGrid::fine(other.npc.pathing.coord.z(), 1),
                );
            }
        }
        masks | NpcInfoProt::FaceCoord as u16
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
        player.build_area.init_nsn_state(1, coord);
        ActivePlayer {
            player,
            handle: Box::new(create_io(IsaacPair::new(&[0; 4], &[0; 4])).handle),
            buffered: Vec::new(),
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
            snap[nid as usize] = NpcSnapshot {
                coord: active.player.pathing.coord.packed(),
                len: renderer.highdefinitions(nid) as u16,
                walk_dir: 1,
                run_dir: if nid % 3 == 0 { 4 } else { -1 },
                flags: NpcSnapshot::PRESENT
                    | NpcSnapshot::ACTIVE
                    | if nid % 3 == 1 { NpcSnapshot::CRAWL } else { 0 },
            };
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
            for (headicons, prayer) in [(0, -1), (8, 0), (16, 1), (32, 2), (56, 2)] {
                active.player.headicons = headicons;
                active.generateappearance(17);
                assert_eq!(active.player.info.last_appearance, Some(17));
                renderer.compute_info(1, &active.player.info, active.player.pathing.coord);
                let bytes = renderer.high_block(1);
                let mut r = Reader { b: bytes, pos: 0 };
                assert_eq!(r.g1(), 0x4);
                assert_eq!(r.g1_alt3() as usize, bytes.len() - 2);
                assert_eq!(r.g1(), gender as i32);
                assert_eq!((r.byte(), r.byte(), r.byte()), (-1, -1, prayer));
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
        npc_info.observer = 5;
        npc_info.highdefinition(&renderer, 9);
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
        player_info.observer = 7;
        player_info.highdefinition(&mut renderer, 7);
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
        player_info.highdefinition(&mut renderer, 7);
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
        other.player.info.last_appearance_info = Some(vec![1, 2, 3].into_boxed_slice());
        other.player.info.masks = PlayerInfoProt::Appearance as u16;
        renderer.compute_info(7, &other.player.info, other.player.pathing.coord);
        renderer.remove_temporary();
        other.player.info.masks = 0;
        other.player.info.face_entity = Some(42);
        other.player.info.orientation_x = Some(CoordGrid::fine(3223, 1));
        other.player.info.orientation_z = Some(CoordGrid::fine(3218, 1));

        let mut info = PlayerInfo::new();
        info.lowdefinition(&mut renderer, &mut active, &other);
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
        player_info.observer = 7;
        player_info.highdefinition(&mut renderer, 7);
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
        player_info.observer = 9;
        player_info.write_blocks(&renderer, 7, info.masks);
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
        player_info.observer = 7;
        player_info.highdefinition(&mut renderer, 7);
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
        player_info.highdefinition(&mut renderer, 7);
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

        let mut seen = |observer: u16| {
            let mut player_info = PlayerInfo::new();
            player_info.observer = observer;
            player_info.highdefinition(&mut renderer, 7);
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
        player_info.observer = 11;
        player_info.highdefinition(&mut renderer, 7);
        let bytes = &player_info.updates.data[..player_info.updates.pos];
        let mut r = Reader { b: bytes, pos: 0 };
        assert_eq!(r.g1(), 0x2);
        let count = r.g1();
        assert_eq!(r.client_hits(count), vec![(9, 40, -1, -1, 0, 255)]);
    }
}
