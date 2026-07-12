use crate::engine::{Engine, cache, engine};
use rs_entity::interaction::InteractionTarget;
use rs_entity::{MoveStrategy, PathingEntity};
use rs_grid::CoordGrid;
use rs_info::Visibility;
use rs_pack::types::BlockWalk;
use rs_vm::engine::ScriptEngine;
use rs_vm::state::{LocRef, ObjRef};
use rs_vm::subject::ScriptSubject;
use rs_zone::zone_map::ZoneMap;
use rsmod::rsmod::flag::collision_flag::CollisionFlag;

/// Identifies an entity by type and slot index for zone and collision
/// tracking operations.
pub(crate) enum EntityId {
    /// An NPC identified by its NID (NPC slot index).
    Npc(u16),
    /// A player identified by its PID (player slot index).
    Player(u16),
}

impl Engine {
    /// Converts an [`InteractionTarget`] to a [`ScriptSubject`] for use
    /// as the secondary subject in RuneScript execution.
    ///
    /// Maps each target variant to its script-VM representation:
    ///
    /// * `Obj` -> `ScriptSubject::Obj` with packed coordinate, ID, and
    ///   count.
    /// * `Npc` -> `ScriptSubject::Npc` with the NPC's UID.
    /// * `Player` -> `ScriptSubject::Player` with the player's UID.
    /// * `Loc` -> `ScriptSubject::Loc` with packed coordinate, ID, shape,
    ///   angle, and layer.
    ///
    /// # Returns
    ///
    /// `None` if the referenced entity no longer exists in the engine.
    pub(crate) fn target_to_subject(target: &InteractionTarget) -> Option<ScriptSubject> {
        match target {
            InteractionTarget::Obj { coord, id, count } => Some(ScriptSubject::Obj(ObjRef {
                coord: *coord,
                id: *id,
                count: *count,
            })),
            InteractionTarget::Npc { nid } => {
                let npc_active = engine().get_npc(*nid)?;
                Some(ScriptSubject::Npc(npc_active.npc.uid))
            }
            InteractionTarget::Player { pid } => {
                let player_active = engine().get_player(*pid)?;
                Some(ScriptSubject::Player(player_active.player.uid))
            }
            InteractionTarget::Loc {
                coord,
                id,
                shape,
                angle,
                layer,
                ..
            } => Some(ScriptSubject::Loc(LocRef {
                coord: *coord,
                id: *id,
                shape: *shape,
                angle: *angle,
                layer: *layer,
            })),
        }
    }

    /// Resolves the current world coordinate of an interaction target.
    ///
    /// For entity targets (Player, Npc), looks up the entity's live
    /// coordinate. For static targets (Obj, Loc), returns the stored
    /// coordinate directly. Returns `(0, 0, 0)` if the entity no longer
    /// exists.
    pub(crate) fn target_coord(target: &InteractionTarget) -> CoordGrid {
        match target {
            InteractionTarget::Player { pid } => engine()
                .get_player(*pid)
                .map(|p| p.player.pathing.coord)
                .unwrap_or(CoordGrid::new(0, 0, 0)),
            InteractionTarget::Npc { nid } => engine()
                .get_npc(*nid)
                .map(|n| n.npc.pathing.coord)
                .unwrap_or(CoordGrid::new(0, 0, 0)),
            InteractionTarget::Obj { coord, .. } => *coord,
            InteractionTarget::Loc { coord, .. } => *coord,
        }
    }

    /// Validates that an interaction target still exists and is on the same
    /// Y-level as the interacting entity.
    ///
    /// Per-variant checks:
    ///
    /// * **Player:** Must exist, be active, share the same Y-level, not be
    ///   logging out, and have default visibility.
    /// * **Npc:** Must exist, be active, share the same Y-level, and (if
    ///   `subject_type` is provided) match the expected NPC type. When
    ///   `allow_delayed_npc` is `false` (player validators) a delayed NPC --
    ///   e.g. one mid death sequence -- is treated as invalid. NPC validators pass
    ///   `true` because NPCs may interact with delayed NPCs.
    /// * **Obj:** Must exist in the zone at the stored coordinate, be
    ///   visible at the current clock, and share the same Y-level. Uses
    ///   `user37` to check receiver-specific visibility.
    /// * **Loc:** Must exist in the zone at the stored coordinate and
    ///   layer, match the expected ID, and share the same Y-level.
    ///
    /// # Returns
    ///
    /// `true` if the target is valid and accessible.
    pub(crate) fn entity_validate_target(
        y: u8,
        target: &InteractionTarget,
        subject_type: Option<u16>,
        user37: Option<u64>,
        allow_delayed_npc: bool,
    ) -> bool {
        match target {
            InteractionTarget::Player { pid } => {
                if let Some(target_player) = engine().get_player(*pid) {
                    if target_player.player.pathing.coord.y() != y || !target_player.player.active {
                        return false;
                    }
                    if target_player.player.logout_sent {
                        return false;
                    }
                    if target_player.player.info.vis != Visibility::Default {
                        return false;
                    }
                    true
                } else {
                    false
                }
            }
            InteractionTarget::Npc { nid } => {
                if let Some(target_npc) = engine().get_npc(*nid) {
                    if target_npc.npc.pathing.coord.y() != y || !target_npc.npc.active {
                        return false;
                    }
                    if !allow_delayed_npc && target_npc.npc.state.delayed {
                        return false;
                    }
                    if let Some(st) = subject_type {
                        if target_npc.npc.uid.id() != st {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            }
            InteractionTarget::Obj { coord, id, .. } => {
                if coord.y() != y {
                    return false;
                }
                let clock = engine().clock();
                let Some(zone) = engine().zones.zone(coord.x(), coord.y(), coord.z()) else {
                    debug_assert!(
                        false,
                        "Zone not found at coord: x={}, y={}, z={}",
                        coord.x(),
                        coord.y(),
                        coord.z()
                    );
                    return false;
                };
                zone.get_obj(coord.x(), coord.z(), *id, user37)
                    .is_some_and(|idx| zone.objs[idx].visible(clock))
            }
            InteractionTarget::Loc {
                coord, id, layer, ..
            } => {
                if coord.y() != y {
                    return false;
                }
                let Some(zone) = engine().zones.zone(coord.x(), coord.y(), coord.z()) else {
                    debug_assert!(
                        false,
                        "Zone not found at coord: x={}, y={}, z={}",
                        coord.x(),
                        coord.y(),
                        coord.z()
                    );
                    return false;
                };
                if let Some(idx) = zone.get_loc_by_layer(coord.x(), coord.z(), *layer) {
                    if zone.locs[idx].id() != *id {
                        return false;
                    }
                    if let Some(st) = subject_type {
                        if zone.locs[idx].id() != st {
                            return false;
                        }
                    }
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Checks whether a pathing entity is within operable (adjacent /
    /// melee) distance of an interaction target.
    ///
    /// Uses `rsmod::reached` to test collision-map reachability. For Obj
    /// targets, checks both shape -2 (default reach) and shape -1
    /// (diagonal reach). For Loc targets, incorporates the loc's
    /// dimensions, shape, angle, and `forceapproach` flag. For entity
    /// targets, uses a default 1x1 or NxN reach check.
    ///
    /// # Returns
    ///
    /// `true` if the entity can operate on the target from its current
    /// position.
    pub(crate) fn entity_in_operable_distance(
        pathing: &PathingEntity,
        target: &InteractionTarget,
    ) -> bool {
        let y = pathing.coord.y();
        let src_x = pathing.coord.x();
        let src_z = pathing.coord.z();
        let size = pathing.size;

        match target {
            InteractionTarget::Obj { coord, .. } => {
                if coord.y() != y {
                    return false;
                }
                rsmod::reached(y, src_x, src_z, coord.x(), coord.z(), 1, 1, size, 0, -2, 0)
                    || rsmod::reached(y, src_x, src_z, coord.x(), coord.z(), 1, 1, size, 0, -1, 0)
            }
            InteractionTarget::Loc {
                coord,
                width,
                length,
                shape,
                angle,
                id,
                ..
            } => {
                if coord.y() != y {
                    return false;
                }
                let forceapproach = cache()
                    .locs
                    .get_by_id(*id)
                    .map(|lt| lt.forceapproach as u8)
                    .unwrap_or(0);
                rsmod::reached(
                    y,
                    src_x,
                    src_z,
                    coord.x(),
                    coord.z(),
                    *width,
                    *length,
                    size,
                    *angle as u8,
                    *shape as i8,
                    forceapproach,
                )
            }
            InteractionTarget::Npc { nid } => {
                if let Some(target_npc) = engine().get_npc(*nid) {
                    if target_npc.npc.pathing.coord.y() != y {
                        return false;
                    }
                    let target_size = target_npc.npc.pathing.size;
                    rsmod::reached(
                        y,
                        src_x,
                        src_z,
                        target_npc.npc.pathing.coord.x(),
                        target_npc.npc.pathing.coord.z(),
                        target_size,
                        target_size,
                        size,
                        0,
                        -2,
                        0,
                    )
                } else {
                    false
                }
            }
            InteractionTarget::Player { pid } => {
                if let Some(player) = engine().get_player(*pid) {
                    if player.player.pathing.coord.y() != y {
                        return false;
                    }
                    rsmod::reached(
                        y,
                        src_x,
                        src_z,
                        player.player.pathing.coord.x(),
                        player.player.pathing.coord.z(),
                        1,
                        1,
                        size,
                        0,
                        -2,
                        0,
                    )
                } else {
                    false
                }
            }
        }
    }

    /// Checks whether a pathing entity is within approach (AP) distance
    /// of an interaction target.
    ///
    /// Approach distance is defined as Chebyshev distance (accounting for
    /// entity sizes) being at most `range` tiles **and** having
    /// line-of-sight to the target. For entity targets (Player, Npc),
    /// also checks that the source and target do not intersect (overlap
    /// is not a valid approach position).
    ///
    /// # Returns
    ///
    /// `true` if the entity is within approach range and has line-of-sight.
    pub(crate) fn entity_in_approach_distance(
        pathing: &PathingEntity,
        target: &InteractionTarget,
        range: i32,
    ) -> bool {
        let y = pathing.coord.y();
        let src_x = pathing.coord.x();
        let src_z = pathing.coord.z();
        let size = pathing.size;

        match target {
            InteractionTarget::Obj { coord, .. } => {
                if coord.y() != y {
                    return false;
                }
                let dist = CoordGrid::distance_to(
                    src_x as i32,
                    src_z as i32,
                    size as i32,
                    size as i32,
                    coord.x() as i32,
                    coord.z() as i32,
                    1,
                    1,
                );
                if dist > range {
                    return false;
                }
                rsmod::has_line_of_sight(
                    y,
                    coord.x(),
                    coord.z(),
                    src_x,
                    src_z,
                    1,
                    1,
                    size,
                    size,
                    CollisionFlag::Player as u32,
                )
            }
            InteractionTarget::Loc {
                coord,
                width,
                length,
                ..
            } => {
                if coord.y() != y {
                    return false;
                }
                let dist = CoordGrid::distance_to(
                    src_x as i32,
                    src_z as i32,
                    size as i32,
                    size as i32,
                    coord.x() as i32,
                    coord.z() as i32,
                    *width as i32,
                    *length as i32,
                );
                if dist > range {
                    return false;
                }
                rsmod::has_line_of_sight(
                    y,
                    coord.x(),
                    coord.z(),
                    src_x,
                    src_z,
                    *width,
                    *length,
                    size,
                    size,
                    CollisionFlag::Player as u32,
                )
            }
            InteractionTarget::Npc { nid } => {
                if let Some(target_npc) = engine().get_npc(*nid) {
                    if target_npc.npc.pathing.coord.y() != y {
                        return false;
                    }
                    let nx = target_npc.npc.pathing.coord.x();
                    let nz = target_npc.npc.pathing.coord.z();
                    let target_size = target_npc.npc.pathing.size;
                    if CoordGrid::intersects(
                        src_x,
                        src_z,
                        size as u16,
                        size as u16,
                        nx,
                        nz,
                        target_size as u16,
                        target_size as u16,
                    ) {
                        return false;
                    }
                    let dist = CoordGrid::distance_to(
                        src_x as i32,
                        src_z as i32,
                        size as i32,
                        size as i32,
                        nx as i32,
                        nz as i32,
                        target_size as i32,
                        target_size as i32,
                    );
                    if dist > range {
                        return false;
                    }
                    rsmod::has_line_of_sight(
                        y,
                        nx,
                        nz,
                        src_x,
                        src_z,
                        target_size,
                        target_size,
                        size,
                        size,
                        CollisionFlag::Player as u32,
                    )
                } else {
                    false
                }
            }
            InteractionTarget::Player { pid } => {
                if let Some(player) = engine().get_player(*pid) {
                    if player.player.pathing.coord.y() != y {
                        return false;
                    }
                    let px = player.player.pathing.coord.x();
                    let pz = player.player.pathing.coord.z();
                    if CoordGrid::intersects(src_x, src_z, size as u16, size as u16, px, pz, 1, 1) {
                        return false;
                    }
                    let dist = CoordGrid::distance_to(
                        src_x as i32,
                        src_z as i32,
                        size as i32,
                        size as i32,
                        px as i32,
                        pz as i32,
                        1,
                        1,
                    );
                    if dist > range {
                        return false;
                    }
                    rsmod::has_line_of_sight(
                        y,
                        px,
                        pz,
                        src_x,
                        src_z,
                        1,
                        1,
                        size,
                        size,
                        CollisionFlag::Player as u32,
                    )
                } else {
                    false
                }
            }
        }
    }

    /// Computes a path from a pathing entity to an interaction target and
    /// queues the resulting waypoints.
    ///
    /// Selects the pathfinding strategy based on the entity's
    /// `move_strategy` and the `client_pathfinder` flag:
    ///
    /// * **Naive / direct overlap:** Uses `rsmod::find_naive_path` for a
    ///   straight-line step. Used when the entity's strategy is `Naive`
    ///   or the client pathfinder reports a coordinate overlap with the
    ///   target.
    /// * **Full A*:** Uses `rsmod::find_path` for a collision-aware route,
    ///   incorporating the target's dimensions, shape, angle, and
    ///   forceapproach configuration for Loc targets.
    ///
    /// The collision type and extra flags are derived from the entity's
    /// `move_restrict` field.
    ///
    /// # Side Effects
    ///
    /// * Queues waypoints on the pathing entity.
    pub(crate) fn entity_path_to_target(
        pathing: &mut PathingEntity,
        target: &InteractionTarget,
        client_pathfinder: bool,
    ) {
        let size = pathing.size;
        let y = pathing.coord.y();
        let x = pathing.coord.x();
        let z = pathing.coord.z();

        let mr = pathing.move_restrict;
        let Some(collision) = PathingEntity::collision_type(mr) else {
            return;
        };
        let extra_flag = PathingEntity::block_walk_extra_flag(mr);

        let naive = pathing.move_strategy == MoveStrategy::Naive;

        match target {
            InteractionTarget::Obj { coord, .. } => {
                if naive || (x == coord.x() && z == coord.z()) {
                    pathing.queue_waypoint(*coord);
                } else {
                    pathing.queue_waypoints(rsmod::find_path(
                        y,
                        x,
                        z,
                        coord.x(),
                        coord.z(),
                        size,
                        1,
                        1,
                        0,
                        -1,
                        true,
                        0,
                        25,
                        collision,
                    ));
                }
            }
            InteractionTarget::Loc {
                coord,
                width,
                length,
                shape,
                angle,
                id,
                ..
            } => {
                if naive {
                    pathing.queue_waypoint(*coord);
                } else {
                    let forceapproach = cache()
                        .locs
                        .get_by_id(*id)
                        .map(|lt| lt.forceapproach as u8)
                        .unwrap_or(0);
                    pathing.queue_waypoints(rsmod::find_path(
                        y,
                        x,
                        z,
                        coord.x(),
                        coord.z(),
                        size,
                        *width,
                        *length,
                        *angle as u8,
                        *shape as i8,
                        true,
                        forceapproach,
                        25,
                        collision,
                    ));
                }
            }
            InteractionTarget::Npc { nid } => {
                if let Some(target_npc) = engine().get_npc(*nid) {
                    let tx = target_npc.npc.pathing.coord.x();
                    let tz = target_npc.npc.pathing.coord.z();
                    let ts = target_npc.npc.pathing.size;
                    if naive
                        || (client_pathfinder
                            && CoordGrid::intersects(
                                x,
                                z,
                                size as u16,
                                size as u16,
                                tx,
                                tz,
                                ts as u16,
                                ts as u16,
                            ))
                    {
                        pathing.queue_waypoints(rsmod::find_naive_path(
                            y, x, z, tx, tz, size, size, ts, ts, extra_flag, collision,
                        ));
                    } else {
                        pathing.queue_waypoints(rsmod::find_path(
                            y, x, z, tx, tz, size, ts, ts, 0, -2, true, 0, 25, collision,
                        ));
                    }
                }
            }
            InteractionTarget::Player { pid } => {
                if let Some(player) = engine().get_player(*pid) {
                    let tx = player.player.pathing.coord.x();
                    let tz = player.player.pathing.coord.z();
                    if naive
                        || (client_pathfinder
                            && CoordGrid::intersects(x, z, size as u16, size as u16, tx, tz, 1, 1))
                    {
                        pathing.queue_waypoints(rsmod::find_naive_path(
                            y, x, z, tx, tz, size, size, 1, 1, extra_flag, collision,
                        ));
                    } else {
                        pathing.queue_waypoints(rsmod::find_path(
                            y, x, z, tx, tz, size, 1, 1, 0, -2, true, 0, 25, collision,
                        ));
                    }
                }
            }
        }
    }

    /// Updates zone membership and collision flags when an entity moves.
    ///
    /// If the entity crossed a zone boundary (different zone X, zone Z,
    /// or Y-level), removes it from the previous zone and adds it to the
    /// new zone.
    ///
    /// If the entity's tile changed at all, updates collision flags
    /// according to the entity's `block_walk` setting:
    ///
    /// * `BlockWalk::Npc` -- toggles NPC collision at old/new positions.
    /// * `BlockWalk::All` -- toggles both NPC and player collision.
    /// * `BlockWalk::None` -- no collision changes.
    ///
    /// # Side Effects
    ///
    /// * Modifies zone player/NPC lists.
    /// * Calls `rsmod::change_npc` and/or `rsmod::change_player` to
    ///   update the collision map.
    pub(crate) fn check_zones_and_collision(
        zones: &mut ZoneMap,
        prev: CoordGrid,
        next: CoordGrid,
        entity: EntityId,
        size: u8,
        block_walk: BlockWalk,
    ) {
        if prev.zone_x() != next.zone_x() || prev.zone_z() != next.zone_z() || prev.y() != next.y()
        {
            match &entity {
                EntityId::Npc(nid) => {
                    zones
                        .zone_mut(prev.x(), prev.y(), prev.z())
                        .remove_npc(*nid);
                    zones.zone_mut(next.x(), next.y(), next.z()).add_npc(*nid);
                }
                EntityId::Player(pid) => {
                    zones
                        .zone_mut(prev.x(), prev.y(), prev.z())
                        .remove_player(*pid);
                    zones
                        .zone_mut(next.x(), next.y(), next.z())
                        .add_player(*pid);
                }
            }
        }
        if prev.x() != next.x() || prev.z() != next.z() || prev.y() != next.y() {
            match block_walk {
                BlockWalk::Npc => {
                    rsmod::change_npc(prev.x(), prev.z(), prev.y(), size, false);
                    rsmod::change_npc(next.x(), next.z(), next.y(), size, true);
                }
                BlockWalk::All => {
                    rsmod::change_npc(prev.x(), prev.z(), prev.y(), size, false);
                    rsmod::change_npc(next.x(), next.z(), next.y(), size, true);
                    rsmod::change_player(prev.x(), prev.z(), prev.y(), size, false);
                    rsmod::change_player(next.x(), next.z(), next.y(), size, true);
                }
                BlockWalk::None => {}
            }
        }
    }
}

/// Extracts a human-readable message from a caught panic payload.
///
/// Attempts to downcast the panic value to `&str` or `String`. Returns
/// `"unknown panic"` if the payload is neither.
///
/// # Returns
///
/// A `Cow<str>` containing the panic message, borrowing from the payload
/// when possible.
pub(crate) fn panic_message(panic: &Box<dyn std::any::Any + Send>) -> std::borrow::Cow<'_, str> {
    if let Some(s) = panic.downcast_ref::<&str>() {
        std::borrow::Cow::Borrowed(*s)
    } else if let Some(s) = panic.downcast_ref::<String>() {
        std::borrow::Cow::Borrowed(s.as_str())
    } else {
        std::borrow::Cow::Borrowed("unknown panic")
    }
}
