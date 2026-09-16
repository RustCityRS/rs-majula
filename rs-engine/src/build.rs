pub use rs_entity::build::*;

use crate::info::{NpcSnapshot, PlayerSnapshot};
use rs_grid::CoordGrid;
use rs_zone::zone_map::ZoneMap;

/// Extension trait for [`BuildArea`] that adds methods requiring access to
/// the live `ActivePlayer` and `ActiveNpc` slices and the [`ZoneMap`].
///
/// These methods perform spatial queries to find nearby entities and filter
/// them by view distance, level, and active state.
pub trait ActiveBuildArea {
    /// Fully rebuilds the tracked player list by clearing existing entries,
    /// resetting view distance to the preferred value, and repopulating
    /// from nearby zones. Reads the per-tick [`PlayerSnapshot`] snapshot rather than the
    /// live player array.
    fn rebuild_players(
        &mut self,
        snap: &[PlayerSnapshot],
        map: &ZoneMap,
        x: u16,
        y: u8,
        z: u16,
        self_pid: u16,
    );

    /// Scans surrounding zones to populate `nearby_players` with player IDs
    /// that pass the visibility filter and are within view distance. Reads the
    /// per-tick [`PlayerSnapshot`] snapshot; `self_pid` is excluded (it is taken out of
    /// the live array during its own encode, so it must not see itself).
    fn get_nearby_players(
        &mut self,
        snap: &[PlayerSnapshot],
        map: &ZoneMap,
        x: u16,
        y: u8,
        z: u16,
        self_pid: u16,
    );

    /// Scans surrounding zones to populate `nearby_npcs` with NPC IDs that
    /// pass the visibility filter and are within the preferred view distance.
    /// Reads the per-tick [`NpcSnapshot`] snapshot.
    fn get_nearby_npcs(&mut self, snap: &[NpcSnapshot], map: &ZoneMap, x: u16, y: u8, z: u16);

    /// Returns `true` if the given player should be included in the nearby
    /// list (not the observer, present, not already tracked, same level, and
    /// within view distance), read from the [`PlayerSnapshot`] snapshot.
    fn filter_player(
        &self,
        snap: &[PlayerSnapshot],
        player: u16,
        self_pid: u16,
        x: u16,
        y: u8,
        z: u16,
    ) -> bool;

    /// Returns `true` if the given NPC should be included in the nearby list
    /// (present, not already tracked, active, same level, and within the
    /// preferred view distance), read from the [`NpcSnapshot`] snapshot.
    fn filter_npc(&self, snap: &[NpcSnapshot], npc: u16, x: u16, y: u8, z: u16) -> bool;
}

impl ActiveBuildArea for BuildArea {
    /// Clears the tracked player set and repopulates it from surrounding zones.
    ///
    /// Resets `last_resize` to 0, restores view distance to
    /// [`BuildArea::PREFERRED_VIEW_DISTANCE`], and then calls
    /// [`get_nearby_players`](ActiveBuildArea::get_nearby_players). If the
    /// nearby count exceeds [`BuildArea::PREFERRED_PLAYERS`], the view distance
    /// is reduced by one tile.
    ///
    /// # Arguments
    /// * `players` - The full world player array.
    /// * `map` - The zone map used for spatial lookup.
    /// * `x` - The observer's X coordinate.
    /// * `y` - The observer's level (floor).
    /// * `z` - The observer's Z coordinate.
    ///
    /// # Side Effects
    /// * Clears and repopulates `self.players` and `self.nearby_players`.
    /// * May reduce `self.view_distance`.
    #[inline]
    fn rebuild_players(
        &mut self,
        snap: &[PlayerSnapshot],
        map: &ZoneMap,
        x: u16,
        y: u8,
        z: u16,
        self_pid: u16,
    ) {
        self.players.clear();
        self.last_resize = 0;
        self.view_distance = BuildArea::PREFERRED_VIEW_DISTANCE;
        self.get_nearby_players(snap, map, x, y, z, self_pid);
        if self.nearby_players.len() >= BuildArea::PREFERRED_PLAYERS as usize {
            self.view_distance -= 1;
        }
    }

    /// Populates `self.nearby_players` with player IDs found in zones within
    /// the current view distance of the given coordinate.
    ///
    /// Iterates over zones in the square region `[x - dist, x + dist]` x
    /// `[z - dist, z + dist]`, collecting players that pass
    /// [`filter_player`](ActiveBuildArea::filter_player). If the total
    /// (nearby + already tracked) exceeds [`BuildArea::PREFERRED_PLAYERS`],
    /// the list is sorted by Euclidean distance and truncated.
    ///
    /// # Arguments
    /// * `players` - The full world player array.
    /// * `map` - The zone map used for spatial lookup.
    /// * `x` - The observer's X coordinate.
    /// * `y` - The observer's level (floor).
    /// * `z` - The observer's Z coordinate.
    ///
    /// # Side Effects
    /// * Clears and repopulates `self.nearby_players`.
    ///
    /// # Safety
    /// Uses unsafe pointer arithmetic on the `players` slice for performance;
    /// callers must guarantee that player IDs from zones are valid indices.
    #[inline]
    fn get_nearby_players(
        &mut self,
        snap: &[PlayerSnapshot],
        map: &ZoneMap,
        x: u16,
        y: u8,
        z: u16,
        self_pid: u16,
    ) {
        self.nearby_players.clear();

        let distance = self.view_distance as u16;
        let start_x = CoordGrid::zone(x.saturating_sub(distance));
        let start_z = CoordGrid::zone(z.saturating_sub(distance));
        let end_x = CoordGrid::zone(x.saturating_add(distance));
        let end_z = CoordGrid::zone(z.saturating_add(distance));
        let cap = BuildArea::PREFERRED_PLAYERS as usize;
        let count = self.players.len();

        'scan: for zx in start_x..=end_x {
            let zone_x = zx << 3;
            for zz in start_z..=end_z {
                if self.nearby_players.len() + count >= cap {
                    break 'scan;
                }
                let zone_z = zz << 3;
                let Some(zone) = map.zone(zone_x, y, zone_z) else {
                    continue;
                };
                for &player_id in zone.players.iter() {
                    if self.filter_player(snap, player_id, self_pid, x, y, z) {
                        self.nearby_players.push(player_id);
                    }
                }
            }
        }

        let k = cap - count;
        if self.nearby_players.len() > k {
            let coord = CoordGrid::new(x, y, z);
            // Keep the `k` closest. `select_nth_unstable_by_key` partitions in
            // O(n) (vs O(n log n) for a full sort) and yields the same SET; the
            // order among the kept players is unspecified, which only affects
            // the order they pop into view, not the tracked iteration order
            // (that stays insertion order via swap_ids/retain_bits).
            if k > 0 {
                self.nearby_players
                    .select_nth_unstable_by_key(k - 1, |&pid| {
                        let s = unsafe { *snap.get_unchecked(pid as usize) };
                        coord.euclidean_squared_distance(CoordGrid::from(s.coord))
                    });
            }
            self.nearby_players.truncate(k);
        }
    }

    /// Populates `self.nearby_npcs` with NPC IDs found in zones within the
    /// preferred view distance of the given coordinate.
    ///
    /// Iterates over zones in the square region, collecting NPCs that pass
    /// [`filter_npc`](ActiveBuildArea::filter_npc). Stops early once the
    /// total (nearby + already tracked) reaches
    /// [`BuildArea::PREFERRED_NPCS`].
    ///
    /// # Arguments
    /// * `npcs` - The full world NPC array.
    /// * `map` - The zone map used for spatial lookup.
    /// * `x` - The observer's X coordinate.
    /// * `y` - The observer's level (floor).
    /// * `z` - The observer's Z coordinate.
    ///
    /// # Side Effects
    /// * Clears and repopulates `self.nearby_npcs`.
    #[inline]
    fn get_nearby_npcs(&mut self, snap: &[NpcSnapshot], map: &ZoneMap, x: u16, y: u8, z: u16) {
        self.nearby_npcs.clear();

        let distance: u16 = BuildArea::PREFERRED_VIEW_DISTANCE as u16;
        let start_x = CoordGrid::zone(x.saturating_sub(distance));
        let start_z = CoordGrid::zone(z.saturating_sub(distance));
        let end_x = CoordGrid::zone(x.saturating_add(distance));
        let end_z = CoordGrid::zone(z.saturating_add(distance));
        let cap = BuildArea::PREFERRED_NPCS as usize;
        let count: usize = self.npcs.len();

        for zx in start_x..=end_x {
            let zone_x: u16 = zx << 3;
            for zz in start_z..=end_z {
                if self.nearby_npcs.len() + count >= cap {
                    return;
                }
                let zone_z: u16 = zz << 3;
                let Some(zone) = map.zone(zone_x, y, zone_z) else {
                    continue;
                };
                for &npc_id in zone.npcs.iter() {
                    if self.nearby_npcs.len() + count >= cap {
                        return;
                    }
                    if self.filter_npc(snap, npc_id, x, y, z) {
                        self.nearby_npcs.push(npc_id);
                    }
                }
            }
        }
    }

    /// Checks whether a player should be included in the nearby list.
    ///
    /// A player passes the filter when it is not already tracked, is on the
    /// same level, and is within the current `view_distance`.
    ///
    /// # Arguments
    /// * `players` - The full world player array.
    /// * `player` - The player index to test.
    /// * `x` - The observer's X coordinate.
    /// * `y` - The observer's level (floor).
    /// * `z` - The observer's Z coordinate.
    ///
    /// # Returns
    /// `true` if the player should be added to the nearby list.
    ///
    /// # Safety
    /// Uses unsafe pointer arithmetic on the `players` slice for performance.
    #[inline]
    fn filter_player(
        &self,
        snap: &[PlayerSnapshot],
        player: u16,
        self_pid: u16,
        x: u16,
        y: u8,
        z: u16,
    ) -> bool {
        if player == self_pid {
            return false;
        }
        let s = unsafe { *snap.get_unchecked(player as usize) };
        let coord = CoordGrid::from(s.coord);
        s.flags & PlayerSnapshot::PRESENT != 0
            && !self.players.contains(player)
            && coord.y() == y
            && CoordGrid::in_distance(&coord, CoordGrid::new(x, y, z), self.view_distance)
    }

    /// Checks whether an NPC should be included in the nearby list.
    ///
    /// An NPC passes the filter when it is not already tracked, is active,
    /// is on the same level, and is within the preferred view distance.
    ///
    /// # Arguments
    /// * `npcs` - The full world NPC array.
    /// * `npc` - The NPC index to test.
    /// * `x` - The observer's X coordinate.
    /// * `y` - The observer's level (floor).
    /// * `z` - The observer's Z coordinate.
    ///
    /// # Returns
    /// `true` if the NPC should be added to the nearby list.
    ///
    /// # Safety
    /// Uses unsafe pointer arithmetic on the `npcs` slice for performance.
    #[inline]
    fn filter_npc(&self, snap: &[NpcSnapshot], npc: u16, x: u16, y: u8, z: u16) -> bool {
        let s = unsafe { *snap.get_unchecked(npc as usize) };
        let coord = CoordGrid::from(s.coord);
        s.flags & NpcSnapshot::PRESENT != 0
            && !self.npcs.contains(npc)
            && s.flags & NpcSnapshot::ACTIVE != 0
            && coord.y() == y
            && CoordGrid::in_distance(
                &coord,
                CoordGrid::new(x, y, z),
                BuildArea::PREFERRED_VIEW_DISTANCE,
            )
    }
}
