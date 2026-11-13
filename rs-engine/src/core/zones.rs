use crate::core::Engine;
use rs_grid::{CoordGrid, ZoneCoordGrid};
use rs_pack::types::LocLayer;
#[cfg(before_647)]
use rsmod::rsmod::flag::zone_flag::ZoneFlag;

/// A deferred zone event scheduled for future execution at a specific game tick.
///
/// These events are stored in `Engine::pending_zone_events` (a `BTreeMap` keyed
/// by clock tick) and processed during the world phase of each game cycle.
///
/// Variants:
/// - `ObjReveal` -- makes a previously receiver-only object visible to all players.
/// - `ObjDelete` -- removes a ground object after its lifetime expires.
/// - `ObjAdd` -- respawns a ground object after it was picked up or removed.
/// - `LocDelete` -- reverts or removes a temporary location change after its duration.
#[derive(PartialEq, Eq)]
pub enum PendingZoneEvent {
    ObjReveal {
        coord: CoordGrid,
        id: u16,
        receiver37: u64,
    },
    ObjDelete {
        coord: CoordGrid,
        id: u16,
        clock: u32,
    },
    ObjAdd {
        coord: CoordGrid,
        id: u16,
    },
    LocDelete {
        coord: CoordGrid,
        layer: LocLayer,
        clock: u32,
    },
}

impl Engine {
    /// Whether `coord` is multi-combat: from 647 the `multiway` area painted
    /// on its tile, before that its zone's `ZoneFlag::Multi`.
    pub fn multiway(&self, coord: CoordGrid) -> bool {
        #[cfg(before_647)]
        return rsmod::is_zone_flagged(coord.x(), coord.z(), coord.y(), ZoneFlag::Multi as u8);
        #[cfg(since_647)]
        return self.cache.multiway_area.is_some_and(|area| {
            let coord = self.area_tile(coord);
            self.cache
                .area_map
                .get(coord.y(), coord.x(), coord.z())
                .contains(&area)
        });
    }

    /// Marks a zone as dirty so that its pending events are flushed to clients
    /// during the zones phase of the current tick.
    ///
    /// # Arguments
    ///
    /// * `x` -- Zone X coordinate.
    /// * `y` -- Zone level (height plane).
    /// * `z` -- Zone Z coordinate.
    ///
    /// # Side Effects
    ///
    /// Inserts the zone coordinate into `self.zones_tracking`.
    pub fn track_zone(&mut self, x: u16, y: u8, z: u16) {
        let coord = ZoneCoordGrid::new(x, y, z);
        self.zones_tracking.insert(coord);
    }

    /// Schedules a [`PendingZoneEvent`] to fire at the given game tick.
    ///
    /// Events are stored in a `BTreeMap<u32, Vec<PendingZoneEvent>>` keyed by
    /// clock tick, ensuring they are processed in chronological order during
    /// the world phase.
    ///
    /// # Arguments
    ///
    /// * `clock` -- The game tick at which the event should be processed.
    /// * `event` -- The zone event to enqueue.
    pub fn schedule_zone_event(&mut self, clock: u32, event: PendingZoneEvent) {
        self.pending_zone_events
            .entry(clock)
            .or_default()
            .push(event);
    }
}
