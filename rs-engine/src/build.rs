pub use rs_entity::build::*;

use crate::active_npc::ActiveNpc;
use crate::info::NpcSnapshot;
#[cfg(before_647)]
use crate::info::PlayerSnapshots;
use rs_grid::CoordGrid;
use rs_zone::zone_map::ZoneMap;

/// [`BuildArea`] queries that read the per-tick snapshots and the [`ZoneMap`]:
/// which players and NPCs are in view.
pub trait ActiveBuildArea {
    /// Untracks every player, then finds the players in view again at the
    /// preferred view distance.
    #[cfg(before_647)]
    fn rebuild_players(
        &mut self,
        snap: &PlayerSnapshots,
        map: &ZoneMap,
        x: u16,
        y: u8,
        z: u16,
        self_pid: u16,
    );

    /// Untracks every NPC, releasing this observer from each. Upstream's rsbuf
    /// only clears the set, so its counts grow on every teleport or level
    /// change.
    fn rebuild_npcs(&mut self, npcs: &mut [Option<Box<ActiveNpc>>]);

    /// Fills `nearby_players` with the untracked players in view, in zone
    /// order, keeping the closest that fit under
    /// [`BuildArea::PREFERRED_PLAYERS`]. The view says who they are, so the
    /// zones are only walked for their order, and only until all are found.
    #[cfg(before_647)]
    fn get_nearby_players(
        &mut self,
        snap: &PlayerSnapshots,
        map: &ZoneMap,
        x: u16,
        y: u8,
        z: u16,
        self_pid: u16,
    );

    /// Fills `nearby_npcs` with untracked NPCs in view, up to
    /// [`BuildArea::PREFERRED_NPCS`] tracked.
    fn get_nearby_npcs(&mut self, snap: &[NpcSnapshot], map: &ZoneMap, x: u16, y: u8, z: u16);

    fn filter_npc(&self, snap: &[NpcSnapshot], npc: u16, x: u16, y: u8, z: u16) -> bool;
}

impl ActiveBuildArea for BuildArea {
    #[cfg(before_647)]
    #[inline]
    fn rebuild_players(
        &mut self,
        snap: &PlayerSnapshots,
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

    #[inline]
    fn rebuild_npcs(&mut self, npcs: &mut [Option<Box<ActiveNpc>>]) {
        for &nid in self.npcs.iter() {
            if let Some(other) = npcs[nid as usize].as_mut() {
                other.npc.observers = other.npc.observers.saturating_sub(1);
            }
        }
        self.npcs.clear();
    }

    #[cfg(before_647)]
    #[inline]
    fn get_nearby_players(
        &mut self,
        snap: &PlayerSnapshots,
        map: &ZoneMap,
        x: u16,
        y: u8,
        z: u16,
        self_pid: u16,
    ) {
        self.nearby_players.clear();
        let mut view = [0u64; MAX_PLAYERS >> 6];
        snap.view(CoordGrid::new(x, y, z), self.view_distance, &mut view);
        view[(self_pid >> 6) as usize] &= !(1 << (self_pid & 63));
        let tracked = self.players.words();
        let mut left = 0;
        for w in 0..MAX_PLAYERS >> 6 {
            view[w] &= !(tracked[w << 1] as u64 | (tracked[(w << 1) + 1] as u64) << 32);
            left += view[w].count_ones();
        }
        if left == 0 {
            return;
        }

        let distance = self.view_distance as u16;
        let start_x = CoordGrid::zone(x.saturating_sub(distance));
        let start_z = CoordGrid::zone(z.saturating_sub(distance));
        let end_x = CoordGrid::zone(x.saturating_add(distance));
        let end_z = CoordGrid::zone(z.saturating_add(distance));
        let room = (BuildArea::PREFERRED_PLAYERS as usize).saturating_sub(self.players.len());

        'scan: for zx in start_x..=end_x {
            let zone_x = zx << 3;
            for zz in start_z..=end_z {
                if self.nearby_players.len() >= room || left == 0 {
                    break 'scan;
                }
                let zone_z = zz << 3;
                let Some(zone) = map.zone(zone_x, y, zone_z) else {
                    continue;
                };
                for &player_id in zone.players.iter() {
                    let (w, bit) = ((player_id >> 6) as usize, 1 << (player_id & 63));
                    if view[w] & bit != 0 {
                        view[w] &= !bit;
                        left -= 1;
                        self.nearby_players.push(player_id);
                    }
                }
            }
        }

        if self.nearby_players.len() > room {
            let coord = CoordGrid::new(x, y, z);
            if room > 0 {
                self.nearby_players
                    .select_nth_unstable_by_key(room - 1, |&pid| {
                        let s = unsafe { *snap.get_unchecked(pid as usize) };
                        coord.euclidean_squared_distance(CoordGrid::from(s.coord))
                    });
            }
            self.nearby_players.truncate(room);
        }
    }

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
