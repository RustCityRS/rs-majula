use crate::active_npc::ActiveNpc;
use crate::active_player::ActivePlayer;
use crate::build::ActiveBuildArea;
use rs_entity::{BuildArea, MAX_NPCS, MAX_PLAYERS};
use rs_grid::CoordGrid;
use rs_info::Visibility;
use rs_info::{NpcRenderer, PlayerRenderer};
use rs_io::Packet;
use rs_io::packet::BitWriter;
use rs_protocol::network::game::info_prot::{NpcInfoProt, PlayerInfoProt};
use rs_zone::zone_map::ZoneMap;

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
/// visibility-filtered). NPC high-def blocks are fully pre-coalesced, so the
/// keep/move path never touches the live `ActiveNpc` at all; only the remove
/// path does, to decrement `observers`.
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

    /// Whether the tracked NPC should be removed from the observer's view this
    /// tick. The `!PRESENT` case (despawned) maps to the old `None` arm (no
    /// observer decrement); the other conditions map to the old
    /// `Some(..) if cond` arm (decrement then remove). NPCs are not
    /// visibility-filtered, so there is no `VIS_HARD`.
    #[inline(always)]
    pub fn should_remove(&self, obs_coord: CoordGrid, obs_y: u8, view_distance: u8) -> bool {
        self.flags & NpcSnapshot::PRESENT == 0
            || self.flags & NpcSnapshot::TELE != 0
            || CoordGrid::from(self.coord).y() != obs_y
            || !CoordGrid::in_distance(&obs_coord, CoordGrid::from(self.coord), view_distance)
            || self.flags & NpcSnapshot::ACTIVE == 0
    }
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
    tracked: Vec<u16>,
    bw: BitWriter,
}

impl PlayerInfo {
    const BITS_ADD: usize = 11 + 5 + 5 + 1 + 1;
    const BITS_RUN: usize = 1 + 2 + 3 + 3 + 1;
    const BITS_WALK: usize = 1 + 2 + 3 + 1;
    const BITS_EXTEND: usize = 1 + 2;
    const BYTES_LIMIT: usize = 5000;

    /// Creates a new `PlayerInfo` encoder with pre-allocated buffers.
    ///
    /// # Returns
    /// A `PlayerInfo` with empty bit/byte buffers sized to
    /// [`BYTES_LIMIT`](Self::BYTES_LIMIT) and a tracked-player vec
    /// pre-allocated to [`BuildArea::PREFERRED_PLAYERS`].
    #[inline(always)]
    pub fn new() -> PlayerInfo {
        PlayerInfo {
            buf: Packet::new(Self::BYTES_LIMIT),
            updates: Packet::new(Self::BYTES_LIMIT),
            tracked: Vec::with_capacity(BuildArea::PREFERRED_PLAYERS as usize),
            bw: BitWriter::new(),
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
    /// A byte slice containing the encoded player info payload, valid until
    /// the next call to `encode`.
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
        players: &[Option<ActivePlayer>],
        snap: &[PlayerSnapshot],
        map: &ZoneMap,
        active: &mut ActivePlayer,
        dx: i32,
        dz: i32,
        rebuild: bool,
    ) -> &[u8] {
        let (px, py, pz) = (
            active.player.pathing.coord.x(),
            active.player.pathing.coord.y(),
            active.player.pathing.coord.z(),
        );
        let self_pid = active.player.uid.pid();
        let build: &mut BuildArea = &mut active.player.build_area;

        if rebuild || dx > build.view_distance as i32 || dz > build.view_distance as i32 {
            build.rebuild_players(snap, map, px, py, pz, self_pid);
        } else {
            build.resize();
        }

        self.updates.pos = 0;
        self.bw.reset();

        self.write_local_player(renderer, active);
        self.write_players(players, snap, renderer, active);
        self.write_new_players(map, players, snap, renderer, active);
        if self.updates.pos > 0 {
            self.bw.pbit::<11>(&mut self.buf, MAX_PLAYERS as i32 - 1);
            self.bw.finish(&mut self.buf);
            self.buf.pdata(&self.updates.data, 0, self.updates.pos);
        } else {
            self.bw.finish(&mut self.buf);
        }
        unsafe { self.buf.data.get_unchecked(0..self.buf.pos) }
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
    fn write_players(
        &mut self,
        players: &[Option<ActivePlayer>],
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
            if s.run_dir != -1 {
                // 1 + 2 + 3 + 3 + 1 = 10 bits (matches `run`)
                let extend = len > 0 && self.fits(PlayerInfo::BITS_RUN, len);
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
                let extend = len > 0 && self.fits(PlayerInfo::BITS_WALK, len);
                self.bw.pbit::<7>(
                    &mut self.buf,
                    (((1 << 2) | 1) << 4) | ((s.walk_dir as i32) << 1) | (extend as i32),
                );
                if extend {
                    self.highdefinition_tracked(players, renderer, active, pid, s.flags);
                }
            } else if len > 0 && self.fits(PlayerInfo::BITS_EXTEND, len) {
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
    fn write_new_players(
        &mut self,
        map: &ZoneMap,
        players: &[Option<ActivePlayer>],
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
            if let Some(other) = unsafe { &*players.as_ptr().add(pid as usize) } {
                if other.player.info.vis == Visibility::Hard {
                    continue;
                }
                let len: usize = renderer.lowdefinitions(pid) + renderer.highdefinitions(pid);
                if !self.fits(PlayerInfo::BITS_ADD, len) {
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
    fn remove(&mut self, active: &mut ActivePlayer, other: u16) {
        // 1 + 2 = 3 bits
        self.bw.pbit::<3>(&mut self.buf, (1 << 2) | 3);
        active.player.build_area.players.remove_bit(other);
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
    fn idle(&mut self) {
        self.bw.pbit::<1>(&mut self.buf, 0);
    }

    /// Writes the high-definition update blocks for a player, masking out
    /// the Chat block when the observer is viewing themselves.
    ///
    /// # Call Stack
    /// **Calls:** [`write_blocks`](Self::write_blocks)
    #[inline(always)]
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
    fn highdefinition_tracked(
        &mut self,
        players: &[Option<ActivePlayer>],
        renderer: &mut PlayerRenderer,
        active: &ActivePlayer,
        pid: u16,
        flags: u8,
    ) {
        if pid == active.player.uid.pid() || flags & PlayerSnapshot::HAS_EXACTMOVE != 0 {
            let other = unsafe {
                (*players.as_ptr().add(pid as usize))
                    .as_ref()
                    .unwrap_unchecked()
            };
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
        active: &mut ActivePlayer,
        other: &ActivePlayer,
    ) {
        let pid = other.player.uid.pid();
        let mut masks = other.player.info.masks;

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

        if let Some(face_entity) = other.player.info.face_entity
            && !renderer.has(pid, PlayerInfoProt::FaceEntity)
        {
            renderer.cache_face_entity(pid, face_entity);
            masks |= PlayerInfoProt::FaceEntity as u16;
        }

        if !renderer.has(pid, PlayerInfoProt::FaceCoord) {
            if let (Some(face_x), Some(face_z)) =
                (other.player.info.face_x, other.player.info.face_z)
            {
                renderer.cache_face_coord(pid, face_x, face_z);
            } else if let (Some(orientation_x), Some(orientation_z)) = (
                other.player.info.orientation_x,
                other.player.info.orientation_z,
            ) {
                renderer.cache_face_coord(pid, orientation_x, orientation_z);
            } else {
                renderer.cache_face_coord(
                    pid,
                    CoordGrid::fine(other.player.pathing.coord.x(), 1),
                    CoordGrid::fine(other.player.pathing.coord.z(), 1),
                );
            }
        }

        masks |= PlayerInfoProt::FaceCoord as u16;

        self.write_blocks(renderer, active, other, masks);
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
    fn write_blocks(
        &mut self,
        renderer: &mut PlayerRenderer,
        active: &ActivePlayer,
        other: &ActivePlayer,
        masks: u16,
    ) {
        let pid = other.player.uid.pid();
        if masks > 0xff {
            self.updates.ip2(masks | PlayerInfoProt::BigInfo as u16);
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

    /// Checks whether the buffer has room for the given number of additional
    /// bits and bytes without exceeding the packet size limit.
    ///
    /// # Arguments
    /// * `bits_to_add` - Number of bits the next movement entry requires.
    /// * `bytes_to_add` - Number of bytes the next update block requires.
    ///
    /// # Returns
    /// `true` if the combined bit buffer (rounded up to bytes) plus the
    /// update buffer plus the new data fits within `BYTES_LIMIT - 3`.
    #[inline(always)]
    fn fits(&self, bits_to_add: usize, bytes_to_add: usize) -> bool {
        ((self.bw.bitpos() + bits_to_add + 7) >> 3) + self.updates.pos + bytes_to_add
            <= Self::BYTES_LIMIT - 3
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
}

impl NpcInfo {
    #[cfg(before_254)]
    const BITS_ADD: usize = 13 + 11 + 5 + 5 + 1;
    #[cfg(since_254)]
    const BITS_ADD: usize = 14 + 11 + 5 + 5 + 1;
    const BITS_RUN: usize = 1 + 2 + 3 + 3 + 1;
    const BITS_WALK: usize = 1 + 2 + 3 + 1;
    const BITS_EXTEND: usize = 1 + 2;
    const BYTES_LIMIT: usize = 5000;

    /// Creates a new `NpcInfo` encoder with pre-allocated buffers.
    ///
    /// # Returns
    /// An `NpcInfo` with empty bit/byte buffers sized to
    /// [`BYTES_LIMIT`](Self::BYTES_LIMIT) and a tracked-NPC vec
    /// pre-allocated to [`BuildArea::PREFERRED_NPCS`].
    #[inline(always)]
    pub fn new() -> NpcInfo {
        NpcInfo {
            buf: Packet::new(Self::BYTES_LIMIT),
            updates: Packet::new(Self::BYTES_LIMIT),
            tracked: Vec::with_capacity(BuildArea::PREFERRED_NPCS as usize),
            bw: BitWriter::new(),
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
    /// A byte slice containing the encoded NPC info payload, valid until
    /// the next call to `encode`.
    ///
    /// # Call Stack
    /// **Calls:** [`write_npcs`](Self::write_npcs),
    /// [`write_new_npcs`](Self::write_new_npcs)
    #[allow(clippy::too_many_arguments)]
    #[inline(always)]
    pub fn encode(
        &mut self,
        renderer: &mut NpcRenderer,
        npcs: &mut [Option<ActiveNpc>],
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
            build.rebuild_npcs();
        }

        self.updates.pos = 0;
        self.bw.reset();

        self.write_npcs(npcs, nsnap, renderer, active);
        self.write_new_npcs(map, npcs, nsnap, renderer, active);
        if self.updates.pos > 0 {
            #[cfg(before_254)]
            self.bw.pbit::<13>(&mut self.buf, MAX_NPCS as i32 - 1);
            #[cfg(since_254)]
            self.bw.pbit::<14>(&mut self.buf, MAX_NPCS as i32 - 1);
            self.bw.finish(&mut self.buf);
            self.buf.pdata(&self.updates.data, 0, self.updates.pos);
        } else {
            self.bw.finish(&mut self.buf);
        }
        unsafe { self.buf.data.get_unchecked(0..self.buf.pos) }
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
        npcs: &mut [Option<ActiveNpc>],
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
            // Movement decision from the compact snapshot. `!PRESENT` reproduces
            // the old `None => remove` (despawned/removed NPC: no observer
            // decrement). A present-but-out-of-view/inactive NPC matches the old
            // `Some(..) if cond` arm: decrement `observers` on the live NPC and
            // remove. The keep/move path never touches the ~1.5 KB ActiveNpc;
            // NPC high-def blocks are fully pre-coalesced so even the extend
            // path only needs `nid`.
            let s = unsafe { *nsnap.get_unchecked(nid as usize) };
            if s.should_remove(obs_coord, obs_y, BuildArea::PREFERRED_VIEW_DISTANCE) {
                // Decrement the observer count only for an NPC that is still
                // present but leaving view (the old `Some(..) if cond` arm). A
                // despawned NPC (`!PRESENT`) maps to the old `None` arm, which
                // did not decrement.
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
            if s.run_dir != -1 {
                // 1 + 2 + 3 + 3 + 1 = 10 bits
                let extend = len > 0 && self.fits(NpcInfo::BITS_RUN, len);
                self.bw.pbit::<10>(
                    &mut self.buf,
                    (((1 << 2) | 2) << 7)
                        | ((s.walk_dir as i32) << 4)
                        | ((s.run_dir as i32) << 1)
                        | (extend as i32),
                );
                if extend {
                    self.highdefinition(renderer, nid);
                }
            } else if s.walk_dir != -1 {
                // 1 + 2 + 3 + 1 = 7 bits
                let extend = len > 0 && self.fits(NpcInfo::BITS_WALK, len);
                self.bw.pbit::<7>(
                    &mut self.buf,
                    (((1 << 2) | 1) << 4) | ((s.walk_dir as i32) << 1) | (extend as i32),
                );
                if extend {
                    self.highdefinition(renderer, nid);
                }
            } else if len > 0 && self.fits(NpcInfo::BITS_EXTEND, len) {
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
        npcs: &mut [Option<ActiveNpc>],
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
                let len: usize = renderer.lowdefinitions(nid) + renderer.highdefinitions(nid);
                if !self.fits(NpcInfo::BITS_ADD, len) {
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
    ) {
        // 13/14 + 11 = 24/25 bits, then 5 + 5 + 1 = 11 bits (35/36 total, split for i32)
        #[cfg(before_254)]
        self.bw
            .pbit::<24>(&mut self.buf, ((nid as i32) << 11) | (ntype as i32));
        #[cfg(since_254)]
        self.bw
            .pbit::<25>(&mut self.buf, ((nid as i32) << 11) | (ntype as i32));
        self.bw
            .pbit::<11>(&mut self.buf, ((x & 0x1F) << 6) | ((z & 0x1F) << 1) | 1);
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
        let blk = renderer.high_block(nid);
        self.updates.pdata(blk, 0, blk.len());
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

        if let Some(face_entity) = other.npc.info.face_entity
            && !renderer.has(nid, NpcInfoProt::FaceEntity)
        {
            renderer.cache_face_entity(nid, face_entity);
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
                renderer.cache_face_coord(
                    nid,
                    CoordGrid::fine(other.npc.pathing.coord.x(), 1),
                    CoordGrid::fine(other.npc.pathing.coord.z(), 1),
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

    /// Checks whether the buffer has room for the given number of additional
    /// bits and bytes without exceeding the NPC info packet size limit.
    ///
    /// # Arguments
    /// * `bits_to_add` - Number of bits the next movement entry requires.
    /// * `bytes_to_add` - Number of bytes the next update block requires.
    ///
    /// # Returns
    /// `true` if the combined bit buffer (rounded up to bytes) plus the
    /// update buffer plus the new data fits within `BYTES_LIMIT - 3`.
    #[inline(always)]
    fn fits(&self, bits_to_add: usize, bytes_to_add: usize) -> bool {
        ((self.bw.bitpos() + bits_to_add + 7) >> 3) + self.updates.pos + bytes_to_add
            <= Self::BYTES_LIMIT - 3
    }
}
