use crate::engine::{Engine, PendingZoneEvent};
use crate::game_map::apply_loc_collision;
use rs_entity::EntityLifeTime;
use std::time::Instant;

impl Engine {
    /// Processes the zone phase of the engine tick cycle.
    ///
    /// Two sub-steps run in sequence:
    ///
    /// 1. [`process_pending_zone_events`] -- resolves timed zone events
    ///    (object reveals, deletions, spawns, and loc reverts) that are due
    ///    on or before the current tick.
    /// 2. [`compute_zone_shared`] -- recomputes shared/encoded zone data
    ///    for every zone that was modified during this tick.
    ///
    /// # Side Effects
    ///
    /// * Modifies zone obj and loc state (reveals, deletes, respawns,
    ///   reverts).
    /// * Updates zone collision maps for loc changes.
    /// * Recomputes shared zone data buffers for client transmission.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::cycle`
    /// **Calls:** `process_pending_zone_events`, `compute_zone_shared`
    pub(crate) fn zones(&mut self) {
        self.process_pending_zone_events();
        self.compute_zone_shared();
    }

    /// Drains and applies all pending zone events whose scheduled tick has
    /// arrived.
    ///
    /// Events are stored in a `BTreeMap` keyed by tick. This method splits
    /// off all entries at or before the current clock and processes them:
    ///
    /// * `ObjReveal` -- makes a previously private object visible to all
    ///   players in the zone.
    /// * `ObjDelete` -- removes an object by its creation clock.
    /// * `ObjAdd` -- respawns a previously removed static object.
    /// * `LocDelete` -- handles loc expiry: despawns temporary locs,
    ///   respawns hidden static locs (restoring collision), or reverts
    ///   changed locs to their original type.
    ///
    /// Each processed event marks its zone as dirty for
    /// [`compute_zone_shared`].
    ///
    /// # Side Effects
    ///
    /// * Mutates zone obj and loc arrays.
    /// * Updates collision maps for loc changes.
    /// * Tracks modified zones in `self.zones_tracking`.
    fn process_pending_zone_events(&mut self) {
        self.forget_appeared_objs();
        if self.pending_zone_events.is_empty() {
            return;
        }
        let due = self.pending_zone_events.split_off(&(self.clock + 1));
        let ready = std::mem::replace(&mut self.pending_zone_events, due);

        // Locs come back only once every expiring loc's collision is cleared:
        // collision flags aren't counted, so a temporary loc over part of the
        // static one it stood in for (the gnome stronghold gate's open halves)
        // despawning after the static one came back would clear that part of
        // its footprint.
        let mut restores = Vec::new();
        for (_, events) in ready {
            for event in events {
                match event {
                    PendingZoneEvent::ObjReveal {
                        coord,
                        id,
                        receiver37,
                    } => {
                        let receiver_pid = self.find_pid_by_user37(receiver37).unwrap_or(0);
                        let (x, y, z) = (coord.x(), coord.y(), coord.z());
                        self.zones
                            .zone_mut(x, y, z)
                            .reveal_obj(x, z, id, receiver37, receiver_pid);
                        self.obj_appeared.insert((coord, id), Instant::now());
                        self.track_zone(x, y, z);
                    }
                    PendingZoneEvent::ObjDelete { coord, id, clock } => {
                        let (x, y, z) = (coord.x(), coord.y(), coord.z());
                        self.zones
                            .zone_mut(x, y, z)
                            .remove_obj_by_clock(x, z, id, clock);
                        self.track_zone(x, y, z);
                    }
                    PendingZoneEvent::ObjAdd { coord, id } => {
                        let (x, y, z) = (coord.x(), coord.y(), coord.z());
                        self.zones.zone_mut(x, y, z).respawn_obj(x, z, id);
                        self.obj_appeared.insert((coord, id), Instant::now());
                        self.track_zone(x, y, z);
                    }
                    PendingZoneEvent::LocDelete {
                        coord,
                        layer,
                        clock,
                    } => {
                        let (x, y, z) = (coord.x(), coord.y(), coord.z());
                        let zone = self.zones.zone_mut(x, y, z);
                        let Some(idx) = zone.locs.iter().position(|loc| {
                            loc.is_at(x, z) && loc.layer() == layer && loc.last_clock() == clock
                        }) else {
                            continue;
                        };
                        let loc = zone.locs[idx];
                        if loc.lifetime() == EntityLifeTime::Despawn {
                            self.remove_loc(coord, layer, loc.serial(), 0);
                        } else if !loc.visible() {
                            restores.push((coord, layer, clock));
                        } else if loc.is_changed() {
                            apply_loc_collision(&loc, coord, false);
                            restores.push((coord, layer, clock));
                        }
                    }
                }
            }
        }

        for (coord, layer, clock) in restores {
            let (x, y, z) = (coord.x(), coord.y(), coord.z());
            let zone = self.zones.zone_mut(x, y, z);
            // Found again: despawning locs above swap-removed theirs.
            let Some(idx) = zone.locs.iter().position(|loc| {
                loc.is_at(x, z) && loc.layer() == layer && loc.last_clock() == clock
            }) else {
                continue;
            };
            if !zone.locs[idx].visible() {
                zone.respawn_loc(idx);
                let reverted = zone.locs[idx];
                apply_loc_collision(&reverted, coord, true);
                self.track_zone(x, y, z);
            } else {
                self.revert_loc(coord, layer);
            }
        }
    }

    /// Recomputes shared (pre-encoded) zone update data for all zones that
    /// were modified during this tick.
    ///
    /// Iterates over `self.zones_tracking` and calls `compute_shared()` on
    /// each zone, which pre-encodes the zone's current state into a
    /// buffer that can be efficiently broadcast to observing clients during
    /// the output phase.
    ///
    /// # Side Effects
    ///
    /// * Rebuilds the shared data buffer on each modified zone.
    fn compute_zone_shared(&mut self) {
        for coord in &self.zones_tracking {
            if let Some(zone) = self.zones.zones.get_mut(coord) {
                zone.compute_shared();
            }
        }
    }
}
