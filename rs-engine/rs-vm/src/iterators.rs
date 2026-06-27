use crate::engine::{ScriptEngine, engine};
use crate::state::{LocRef, NpcRef, ObjRef};
use rs_grid::CoordGrid;
use rs_pack::types::HuntCheckVis;

/// Holds the result set and iteration cursor for NPC search operations.
///
/// Populated by functions like [`npc_zone`], [`npc_distance`], or [`npc_distance_any`],
/// then consumed one element at a time via the `cursor` field during script execution
/// of `npc_findnext`-style opcodes.
#[derive(Debug, Clone, Default)]
pub struct NpcIteratorState {
    pub matches: Vec<NpcRef>,
    pub cursor: usize,
}

/// Holds the result set and iteration cursor for location (scenery) search operations.
///
/// Populated by [`loc_zone`] and consumed one element at a time via the `cursor`
/// field during script execution of `loc_findnext`-style opcodes.
#[derive(Debug, Clone, Default)]
pub struct LocIteratorState {
    pub matches: Vec<LocRef>,
    pub cursor: usize,
}

/// Holds the result set and iteration cursor for ground object/item search operations.
///
/// Populated by object search functions and consumed one element at a time via the
/// `cursor` field during script execution of `obj_findnext`-style opcodes.
#[derive(Debug, Clone, Default)]
pub struct ObjIteratorState {
    pub matches: Vec<ObjRef>,
    pub cursor: usize,
}

/// Holds the result set and iteration cursor for player search (hunt) operations.
///
/// Populated by [`hunt_players`] and consumed one element at a time via the `cursor`
/// field during script execution of `player_findnext`-style opcodes. Stores player
/// IDs (`pid`) rather than full `PlayerUid` values.
#[derive(Debug, Clone, Default)]
pub struct PlayerIteratorState {
    pub matches: Vec<u16>,
    pub cursor: usize,
}

/// Collects all location (scenery) references in the zone containing the given coordinate.
///
/// # Arguments
/// * `coord` - A [`CoordGrid`] whose zone-level coordinates identify the target zone.
///
/// # Returns
/// A `Vec<LocRef>` of all locations present in that zone.
///
/// # Call Stack
/// **Calls:** [`ScriptEngine::get_zone_locs`] via the global engine accessor.
pub fn loc_zone<E: ScriptEngine + 'static>(coord: CoordGrid) -> Vec<LocRef> {
    engine::<E>().get_zone_locs(coord.x(), coord.y(), coord.z())
}

/// Collects all ground object references in the zone containing the given coordinate.
pub fn obj_zone<E: ScriptEngine + 'static>(coord: CoordGrid) -> Vec<ObjRef> {
    engine::<E>().get_zone_objs(coord.x(), coord.y(), coord.z())
}

/// Collects all NPC references in the zone containing the given coordinate.
///
/// Corresponds to `NpcIteratorType::ZONE`.
///
/// # Arguments
/// * `coord` - A [`CoordGrid`] whose zone-level coordinates identify the target zone.
///
/// # Returns
/// A `Vec<NpcRef>` of all NPCs present in that zone.
///
/// # Call Stack
/// **Calls:** [`ScriptEngine::get_zone_npcs`] via the global engine accessor.
pub fn npc_zone<E: ScriptEngine + 'static>(coord: CoordGrid) -> Vec<NpcRef> {
    engine::<E>().get_zone_npcs(coord.x(), coord.y(), coord.z())
}

/// Returns the pids of all players in the zone containing `coord`.
pub fn player_zone<E: ScriptEngine + 'static>(coord: CoordGrid) -> Vec<u16> {
    engine::<E>()
        .get_zone_player_pids(coord.x(), coord.y(), coord.z())
        .to_vec()
}

/// Internal implementation for distance-based NPC searches.
///
/// Scans all zones within a radius derived from the Chebyshev distance, filters NPCs
/// by distance, optional type ID, and visibility check (line-of-sight or line-of-walk).
///
/// # Arguments
/// * `id` - If `Some(id)`, only NPCs with a matching type ID are included. If `None`,
///   all NPCs within range are collected regardless of type.
/// * `coord` - The center coordinate for the search.
/// * `distance` - The maximum Chebyshev distance from `coord`.
/// * `vis` - The visibility check mode: line-of-sight, line-of-walk, or off.
///
/// # Returns
/// A `Vec<NpcRef>` of NPCs matching all criteria, iterated in reverse zone order.
///
/// # Call Stack
/// **Called by:** [`npc_distance`], [`npc_distance_any`]
/// **Calls:** [`ScriptEngine::get_zone_npcs`], [`rsmod::has_line_of_sight`],
/// [`rsmod::has_line_of_walk`]
fn npc_distance_inner<E: ScriptEngine + 'static>(
    id: Option<u16>,
    coord: CoordGrid,
    distance: i32,
    vis: HuntCheckVis,
) -> Vec<NpcRef> {
    let center_zx = CoordGrid::zone(coord.x()) as i32;
    let center_zz = CoordGrid::zone(coord.z()) as i32;
    let radius = 1 + (distance >> 3);

    let engine = engine::<E>();
    let mut matches = Vec::new();

    for zx in ((center_zx - radius)..=(center_zx + radius)).rev() {
        for zz in ((center_zz - radius)..=(center_zz + radius)).rev() {
            if zx < 0 || zz < 0 {
                continue;
            }
            let npcs = engine.get_zone_npcs((zx as u16) << 3, coord.y(), (zz as u16) << 3);
            for npc_ref in npcs {
                let npc_coord = CoordGrid::from(npc_ref.coord);
                if coord.distance(npc_coord) > distance {
                    continue;
                }
                if let Some(id) = id
                    && npc_ref.id != id
                {
                    continue;
                }
                match vis {
                    HuntCheckVis::LineOfSight => {
                        if !engine.lineofsight(coord, npc_coord) {
                            continue;
                        }
                    }
                    HuntCheckVis::LineOfWalk => {
                        if !engine.lineofwalk(coord, npc_coord) {
                            continue;
                        }
                    }
                    HuntCheckVis::Off => {}
                }
                matches.push(npc_ref);
            }
        }
    }

    matches
}

/// Finds all NPCs of a specific type within Chebyshev distance of a coordinate.
///
/// Corresponds to `NpcIteratorType::DISTANCE`.
///
/// # Arguments
/// * `id` - The NPC type/config ID to filter by.
/// * `coord` - The center coordinate for the search.
/// * `distance` - The maximum Chebyshev distance from `coord`.
/// * `vis` - The visibility check mode: line-of-sight, line-of-walk, or off.
///
/// # Returns
/// A `Vec<NpcRef>` of NPCs with the given type ID that are within range and pass
/// the visibility check.
///
/// # Call Stack
/// **Calls:** [`npc_distance_inner`]
pub fn npc_distance<E: ScriptEngine + 'static>(
    id: u16,
    coord: CoordGrid,
    distance: i32,
    vis: HuntCheckVis,
) -> Vec<NpcRef> {
    npc_distance_inner::<E>(Some(id), coord, distance, vis)
}

/// Finds all NPCs of any type within Chebyshev distance of a coordinate.
///
/// Corresponds to `NpcIteratorType::DISTANCE` with no type filter.
///
/// # Arguments
/// * `coord` - The center coordinate for the search.
/// * `distance` - The maximum Chebyshev distance from `coord`.
/// * `vis` - The visibility check mode: line-of-sight, line-of-walk, or off.
///
/// # Returns
/// A `Vec<NpcRef>` of all NPCs within range that pass the visibility check,
/// regardless of NPC type.
///
/// # Call Stack
/// **Calls:** [`npc_distance_inner`]
pub fn npc_distance_any<E: ScriptEngine + 'static>(
    coord: CoordGrid,
    distance: i32,
    vis: HuntCheckVis,
) -> Vec<NpcRef> {
    npc_distance_inner::<E>(None, coord, distance, vis)
}

/// Finds all players within Chebyshev distance of a coordinate, with optional visibility filtering.
///
/// Corresponds to `HuntModeType::PLAYER`. Scans zones in a radius derived from the
/// distance, then filters individual players by exact Chebyshev distance and the
/// chosen visibility check.
///
/// # Arguments
/// * `coord` - The center coordinate for the search.
/// * `distance` - The maximum Chebyshev distance from `coord`.
/// * `vis` - The visibility check mode: line-of-sight, line-of-walk, or off.
///
/// # Returns
/// A `Vec<u16>` of player IDs (`pid` values) for players within range that pass
/// the visibility check.
///
/// # Call Stack
/// **Calls:** [`ScriptEngine::get_zone_player_pids`],
/// [`ScriptEngine::get_zone_player_coords`], [`rsmod::has_line_of_sight`],
/// [`rsmod::has_line_of_walk`]
pub fn hunt_players<E: ScriptEngine + 'static>(
    coord: CoordGrid,
    distance: i32,
    vis: HuntCheckVis,
) -> Vec<u16> {
    let center_zx = CoordGrid::zone(coord.x()) as i32;
    let center_zz = CoordGrid::zone(coord.z()) as i32;
    let radius = 1 + (distance >> 3);

    let engine = engine::<E>();
    let mut matches = Vec::new();

    for zx in ((center_zx - radius)..=(center_zx + radius)).rev() {
        for zz in ((center_zz - radius)..=(center_zz + radius)).rev() {
            if zx < 0 || zz < 0 {
                continue;
            }
            let zone_x = (zx as u16) << 3;
            let zone_z = (zz as u16) << 3;
            let pids = engine.get_zone_player_pids(zone_x, coord.y(), zone_z);
            let coords = engine.get_zone_player_coords(zone_x, coord.y(), zone_z);
            for (i, &pid) in pids.iter().enumerate() {
                let player_coord = CoordGrid::from(coords[i]);
                if coord.distance(player_coord) > distance {
                    continue;
                }
                match vis {
                    HuntCheckVis::LineOfSight => {
                        if !engine.lineofsight(coord, player_coord) {
                            continue;
                        }
                    }
                    HuntCheckVis::LineOfWalk => {
                        if !engine.lineofwalk(coord, player_coord) {
                            continue;
                        }
                    }
                    HuntCheckVis::Off => {}
                }
                matches.push(pid);
            }
        }
    }

    matches
}
