use crate::core::{Engine, PendingZoneEvent, lifecycle_clock};
use crate::map::apply_loc_collision;
use rs_entity::{EntityLifeTime, Loc, LocSerial};
use rs_grid::CoordGrid;
use rs_pack::types::{LocAngle, LocLayer, LocShape};
use rs_vm::ScriptError;

impl Engine {
    /// Adds a new loc or changes the existing loc at the given coordinate and layer.
    ///
    /// If a loc is already stored on the same layer at the target tile (a removed
    /// map loc included, which comes back), it is changed in place by
    /// [`change_loc_at`](Self::change_loc_at) and keeps its serial. Otherwise a
    /// brand-new `Despawn` loc is created with a new serial.
    ///
    /// When `duration > 0`, a `LocDelete` event is scheduled to revert or remove
    /// the loc after the specified number of ticks.
    ///
    /// # Arguments
    ///
    /// * `coord` -- World coordinate for the loc.
    /// * `id` -- Loc type id (indexes into `cache.locs`).
    /// * `shape` -- The visual/collision shape of the loc.
    /// * `angle` -- Rotation angle of the loc (0-3).
    /// * `width`, `length`, `blockwalk`, `blockrange` -- From the loc type.
    /// * `duration` -- Lifetime in game ticks before automatic revert (0 = permanent).
    ///
    /// # Returns
    ///
    /// The added or changed loc.
    ///
    /// # Side Effects
    ///
    /// - Updates the collision map (removes old collision, applies new collision).
    /// - Inserts or updates the loc in the zone's loc list.
    /// - Queues a zone change event for client updates.
    /// - Schedules a `LocDelete` event if the loc is temporary.
    /// - Marks the zone as dirty.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `ScriptEngine::add_or_change_loc`, loc interaction scripts.
    /// **Calls:** `change_loc_at`, `apply_loc_collision`, `schedule_zone_event`,
    /// `track_zone`.
    #[allow(clippy::too_many_arguments)]
    pub fn add_or_change_loc(
        &mut self,
        coord: CoordGrid,
        id: u16,
        shape: LocShape,
        angle: LocAngle,
        width: u8,
        length: u8,
        blockwalk: bool,
        blockrange: bool,
        duration: u64,
    ) -> Loc {
        let layer = shape.layer();
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let zone = self.zones.zone_mut(x, y, z);

        let existing = zone
            .locs
            .iter()
            .position(|loc| loc.is_at(x, z) && loc.layer() == layer);

        if let Some(idx) = existing {
            return self.change_loc_at(
                coord, idx, id, shape, angle, width, length, blockwalk, blockrange, duration,
            );
        }

        let loc = Loc::new(
            coord,
            EntityLifeTime::Despawn,
            id,
            shape,
            angle,
            blockwalk,
            blockrange,
            width,
            length,
        );

        apply_loc_collision(&loc, coord, true);

        // `add_loc` evicts the oldest despawn loc when the zone is at the loc
        // cap; clear that evicted loc's collision (owned by the engine, not
        // the zone) at its own tile.
        if let Some(evicted) = zone.add_loc(loc) {
            apply_loc_collision(&evicted, evicted.world_coord(zone.coord), false);
        }

        // `add_loc` pushed the new loc last, with its serial.
        let added = zone.locs.len() - 1;
        if duration > 0 {
            let clock = lifecycle_clock(self.clock, duration);
            zone.locs[added].set_last_clock(clock);
            self.schedule_zone_event(
                clock,
                PendingZoneEvent::LocDelete {
                    coord,
                    layer,
                    clock,
                },
            );
        }
        self.track_zone(x, y, z);
        self.zones.zone_mut(x, y, z).locs[added]
    }

    /// Changes the exact loc with `serial` on `layer` at `coord` to type `id`,
    /// keeping its shape and angle (`loc_change`).
    ///
    /// A removed map loc comes back as the new type, as in the reference engine;
    /// a loc a script added that is gone is left alone.
    ///
    /// # Returns
    ///
    /// The loc as changed, or `None` if it is gone.
    #[allow(clippy::too_many_arguments)]
    pub fn change_loc(
        &mut self,
        coord: CoordGrid,
        layer: LocLayer,
        serial: LocSerial,
        id: u16,
        width: u8,
        length: u8,
        blockwalk: bool,
        blockrange: bool,
        duration: u64,
    ) -> Option<Loc> {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let zone = self.zones.zone_mut(x, y, z);
        let idx = zone.loc_index(x, z, layer, serial)?;
        let loc = zone.locs[idx];
        Some(self.change_loc_at(
            coord,
            idx,
            id,
            loc.shape(),
            loc.angle(),
            width,
            length,
            blockwalk,
            blockrange,
            duration,
        ))
    }

    /// Changes the stored loc `locs[idx]` of the zone holding `coord` in place.
    ///
    /// The old collision is removed if the loc was shown, its type, shape, angle,
    /// flags and size are updated, the new collision is applied and the loc is
    /// shown. A loc left changed, or any loc a script added, gets a `LocDelete`
    /// timer for `duration`; a map loc changed back to its own type gets none.
    ///
    /// # Returns
    ///
    /// The changed loc, which keeps its serial.
    #[allow(clippy::too_many_arguments)]
    fn change_loc_at(
        &mut self,
        coord: CoordGrid,
        idx: usize,
        id: u16,
        shape: LocShape,
        angle: LocAngle,
        width: u8,
        length: u8,
        blockwalk: bool,
        blockrange: bool,
        duration: u64,
    ) -> Loc {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let zone = self.zones.zone_mut(x, y, z);

        let old_loc = zone.locs[idx];
        let layer = old_loc.layer();
        if old_loc.visible() {
            apply_loc_collision(&old_loc, coord, false);
        }

        zone.locs[idx].change(id, shape, angle, blockwalk, blockrange, width, length);

        // Apply collision from the *stored* loc. The change above stored the new
        // id/angle/flags; only shape is shared (a change can't alter it). Reading
        // the loc back keeps the footprint added here identical to the one a later
        // revert/remove will remove from the same loc.
        let updated = zone.locs[idx];
        apply_loc_collision(&updated, coord, true);

        zone.change_loc(idx);

        let is_changed = zone.locs[idx].is_changed();
        let is_despawn = zone.locs[idx].lifetime() == EntityLifeTime::Despawn;
        if is_changed || is_despawn {
            let clock = lifecycle_clock(self.clock, duration);
            zone.locs[idx].set_last_clock(clock);
            self.schedule_zone_event(
                clock,
                PendingZoneEvent::LocDelete {
                    coord,
                    layer,
                    clock,
                },
            );
        } else {
            zone.locs[idx].set_last_clock(u32::MAX);
        }
        self.track_zone(x, y, z);
        self.zones.zone_mut(x, y, z).locs[idx]
    }

    /// Looks up loc type `id` for placing at `coord` with `angle`, checking that
    /// every zone its footprint covers exists.
    ///
    /// # Returns
    ///
    /// The type's `(width, length, blockwalk, blockrange)`.
    pub(crate) fn placeable_loc_type(
        &self,
        coord: CoordGrid,
        id: u16,
        angle: LocAngle,
    ) -> rs_vm::Result<(u8, u8, bool, bool)> {
        let loc_type = self
            .cache
            .locs
            .get_by_id(id)
            .ok_or(ScriptError::LocNotFound(id as i32))?;
        let (size_x, size_z) = match angle {
            LocAngle::North | LocAngle::South => (loc_type.length, loc_type.width),
            _ => (loc_type.width, loc_type.length),
        };
        let (max_x, max_z) = (
            coord.x() + size_x.max(1) as u16 - 1,
            coord.z() + size_z.max(1) as u16 - 1,
        );
        for zone_x in CoordGrid::zone(coord.x())..=CoordGrid::zone(max_x) {
            for zone_z in CoordGrid::zone(coord.z())..=CoordGrid::zone(max_z) {
                if !rsmod::is_zone_allocated(zone_x << 3, zone_z << 3, coord.y()) {
                    return Err(ScriptError::Runtime(format!(
                        "Zone does not exist at coord: {:?}",
                        CoordGrid::new(zone_x << 3, coord.y(), zone_z << 3)
                    )));
                }
            }
        }
        Ok((
            loc_type.width,
            loc_type.length,
            loc_type.blockwalk,
            loc_type.blockrange,
        ))
    }

    /// Removes the exact loc with `serial` on `layer` at `coord`, if it is shown.
    ///
    /// Clears the loc's collision from the pathfinding map and removes it from
    /// the zone. A loc a script added is gone for good. A map (`Respawn`) loc
    /// stays stored, hidden and keeping its current type; with a non-zero
    /// `duration` a `LocDelete` event respawns it after that many ticks, and with
    /// zero it stays removed.
    ///
    /// # Arguments
    ///
    /// * `coord` -- World coordinate of the loc.
    /// * `layer` -- The loc layer to target (e.g. wall, ground decoration).
    /// * `serial` -- The serial of the loc to remove.
    /// * `duration` -- Respawn delay in game ticks (0 = never).
    ///
    /// # Side Effects
    ///
    /// - Removes collision for the loc from the pathfinding map.
    /// - Marks the loc as removed in the zone.
    /// - Schedules a `LocDelete` respawn event if applicable.
    /// - Marks the zone as dirty.
    ///
    /// # Call Stack
    ///
    /// **Called by:** loc interaction scripts, the zone phase when a despawn
    /// timer expires.
    /// **Calls:** `apply_loc_collision`, `schedule_zone_event`, `track_zone`.
    pub fn remove_loc(
        &mut self,
        coord: CoordGrid,
        layer: LocLayer,
        serial: LocSerial,
        duration: u64,
    ) {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let zone = self.zones.zone_mut(x, y, z);

        let Some(idx) = zone.loc_index(x, z, layer, serial) else {
            return;
        };

        // Copy the loc out before removal: `remove_loc` removes despawn locs
        // (shrinking `locs`), so `zone.locs[idx]` is invalid afterward. Respawn
        // locs stay in place, so indexing `locs[idx]` stays valid in the
        // respawn branch below.
        let loc = zone.locs[idx];
        if !loc.visible() {
            return;
        }

        apply_loc_collision(&loc, coord, false);

        zone.remove_loc(idx);

        if loc.lifetime() == EntityLifeTime::Respawn {
            // A pending change timer must not bring the loc back early.
            let clock = if duration > 0 {
                lifecycle_clock(self.clock, duration)
            } else {
                u32::MAX
            };
            zone.locs[idx].set_last_clock(clock);
            if duration > 0 {
                self.schedule_zone_event(
                    clock,
                    PendingZoneEvent::LocDelete {
                        coord,
                        layer,
                        clock,
                    },
                );
            }
        }
        self.track_zone(x, y, z);
    }

    /// Reverts a loc at the given coordinate and layer to its original state.
    ///
    /// Undoes any `change` that was applied to the loc, restoring its original
    /// type, shape, and angle, and applies the reverted loc's collision. The
    /// caller has already cleared the changed loc's collision (see
    /// `process_pending_zone_events`). Clears the loc's `last_clock` so no
    /// further timed events will fire for it.
    ///
    /// # Arguments
    ///
    /// * `coord` -- World coordinate of the loc.
    /// * `layer` -- The loc layer to target.
    ///
    /// # Side Effects
    ///
    /// - Applies the reverted loc's collision.
    /// - Reverts the loc's properties in the zone.
    /// - Queues a zone change event for client updates.
    /// - Marks the zone as dirty.
    pub fn revert_loc(&mut self, coord: CoordGrid, layer: LocLayer) {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        let zone = self.zones.zone_mut(x, y, z);

        let Some(idx) = zone.get_loc_by_layer(x, z, layer) else {
            return;
        };

        zone.locs[idx].revert();

        let reverted = zone.locs[idx];
        apply_loc_collision(&reverted, coord, true);

        zone.change_loc(idx);
        zone.locs[idx].set_last_clock(u32::MAX);
        self.track_zone(x, y, z);
    }

    /// Scans the target zone and its three south-west neighbors for an active
    /// loc whose footprint covers `coord`, returning `true` on the first hit.
    ///
    /// When `skip_invisible_walls` is set, invisible wall locs are ignored
    /// (what `locaddunsafe` wants); `map_loc` passes `false` so they still
    /// count.
    pub(crate) fn loc_occupies(&self, coord: CoordGrid, skip_invisible_walls: bool) -> bool {
        let (x, y, z) = (coord.x(), coord.y(), coord.z());
        for dx in (-8..=0).step_by(8) {
            for dz in (-8..=0).step_by(8) {
                let zx = x as i32 + dx;
                let zz = z as i32 + dz;
                if zx < 0 || zz < 0 {
                    continue;
                }
                let Some(zone) = self.zones.zone(zx as u16, y, zz as u16) else {
                    continue;
                };
                for loc in &zone.locs {
                    let Some(loc_type) = self.cache.locs.get_by_id(loc.id()) else {
                        continue;
                    };

                    if loc_type.active != Some(true) {
                        continue;
                    }

                    if skip_invisible_walls && !loc.visible() && loc.layer() == LocLayer::Wall {
                        continue;
                    }

                    let loc_coord = loc.world_coord(zone.coord);
                    let (width, length) = match loc.angle() {
                        LocAngle::North | LocAngle::South => (loc.length(), loc.width()),
                        _ => (loc.width(), loc.length()),
                    };

                    for index in 0..(width as u16 * length as u16) {
                        let lx = loc_coord.x() + (index % width as u16);
                        let lz = loc_coord.z() + (index / width as u16);
                        if lx == x && lz == z {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}
