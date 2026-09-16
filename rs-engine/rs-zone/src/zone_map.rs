use crate::zone::Zone;
use rs_entity::{Loc, Obj};
use rs_grid::ZoneCoordGrid;
use rustc_hash::FxHashMap;

/// Global lookup table mapping zone coordinates to their [`Zone`] instances.
///
/// Stored in the `Engine` struct, `ZoneMap` provides O(1) access to any zone
/// by its (x, y, z) coordinate triple. Zones are lazily created on first
/// mutable access via [`zone_mut`](Self::zone_mut).
///
/// The `Zone` values are boxed so the hash map's `(key, value)` storage holds
/// only a small pointer per slot (~12 B) instead of a full inline `Zone`
/// (~170 B). With thousands of loaded zones this keeps the probe array
/// cache-resident, so the per-zone lookups in the info phase
/// (`get_nearby_*`, `update_zones`) stay cheap.
pub struct ZoneMap {
    pub zones: FxHashMap<ZoneCoordGrid, Box<Zone>>,
}

impl ZoneMap {
    /// Creates a new, empty `ZoneMap`.
    ///
    /// # Returns
    ///
    /// A `ZoneMap` with no zones allocated. Zones are created lazily via
    /// [`zone_mut`](Self::zone_mut).
    ///
    /// **Called by:** `Engine::new` during server startup.
    #[inline]
    #[allow(clippy::new_without_default)]
    pub fn new() -> ZoneMap {
        ZoneMap {
            zones: FxHashMap::default(),
        }
    }

    /// Returns an immutable reference to the zone at the given coordinates, if it exists.
    ///
    /// # Arguments
    ///
    /// * `x` -- The zone x coordinate.
    /// * `y` -- The zone y (level/plane) coordinate.
    /// * `z` -- The zone z coordinate.
    ///
    /// # Returns
    ///
    /// `Some(&Zone)` if the zone has been previously created, `None` otherwise.
    ///
    /// **Called by:** `Engine` methods, `ActivePlayer::update_zones`,
    /// `BuildArea` neighbour scanning, `InfoProtocol` queries, op handlers.
    #[inline]
    pub fn zone(&self, x: u16, y: u8, z: u16) -> Option<&Zone> {
        let coord = ZoneCoordGrid::new(x, y, z);
        self.zones.get(&coord).map(|z| &**z)
    }

    /// Returns a mutable reference to the zone at the given coordinates, creating it if absent.
    ///
    /// This is the primary entry point for zone mutation. If no zone exists at
    /// the given coordinates, a new empty [`Zone`] is inserted and returned.
    ///
    /// # Arguments
    ///
    /// * `x` -- The zone x coordinate.
    /// * `y` -- The zone y (level/plane) coordinate.
    /// * `z` -- The zone z coordinate.
    ///
    /// # Returns
    ///
    /// A mutable reference to the (possibly newly created) `Zone`.
    ///
    /// **Called by:** `Engine` methods for entity placement, `GameMap::load` for
    /// static loc/obj population, `ActivePlayer::update_zones`, zone phase processing.
    #[inline]
    pub fn zone_mut(&mut self, x: u16, y: u8, z: u16) -> &mut Zone {
        let coord = ZoneCoordGrid::new(x, y, z);
        // `or_insert_with` so the `Zone` is only constructed on actual insert
        // (the eager `or_insert` built one every call even when present).
        self.zones
            .entry(coord)
            .or_insert_with(|| Box::new(Zone::new(coord)))
    }

    /// Resolves the zone holding a tile and the index of a matching location in it.
    ///
    /// This is the shared form of the "look up the zone, then index into it"
    /// pattern. Callers that need the zone itself -- to reach `Zone::coord` for
    /// [`Loc::world_coord`], say -- use this; callers that only want the location
    /// use [`loc_at`](Self::loc_at).
    ///
    /// # Arguments
    ///
    /// * `x` -- The tile x coordinate.
    /// * `y` -- The level (height plane).
    /// * `z` -- The tile z coordinate.
    /// * `id` -- The location type id to match.
    ///
    /// # Returns
    ///
    /// `Some((zone, index))` addressing `zone.locs[index]`, or `None` if the zone
    /// is not loaded or holds no such location.
    #[inline]
    pub fn find_loc(&self, x: u16, y: u8, z: u16, id: u16) -> Option<(&Zone, usize)> {
        let zone = self.zone(x, y, z)?;
        let idx = zone.get_loc(x, z, id)?;
        Some((zone, idx))
    }

    /// Returns the placed location matching `id` at the given tile.
    ///
    /// # Arguments
    ///
    /// * `x` -- The tile x coordinate.
    /// * `y` -- The level (height plane).
    /// * `z` -- The tile z coordinate.
    /// * `id` -- The location type id to match.
    ///
    /// # Returns
    ///
    /// `Some(&Loc)`, or `None` if the zone is not loaded or holds no such location.
    #[inline]
    pub fn loc_at(&self, x: u16, y: u8, z: u16, id: u16) -> Option<&Loc> {
        let (zone, idx) = self.find_loc(x, y, z, id)?;
        Some(&zone.locs[idx])
    }

    /// Resolves the zone holding a tile and the index of a matching ground object.
    ///
    /// The counterpart of [`find_loc`](Self::find_loc) for ground objects; callers
    /// that only want the object use [`obj_at`](Self::obj_at).
    ///
    /// # Arguments
    ///
    /// * `x` -- The tile x coordinate.
    /// * `y` -- The level (height plane).
    /// * `z` -- The tile z coordinate.
    /// * `id` -- The object type id to match.
    /// * `receiver37` -- Base-37 username to match receiver-only objects against,
    ///   or `None` to consider only objects visible to everyone.
    ///
    /// # Returns
    ///
    /// `Some((zone, index))` addressing `zone.objs[index]`, or `None` if the zone
    /// is not loaded or holds no such object visible to this receiver.
    #[inline]
    pub fn find_obj(
        &self,
        x: u16,
        y: u8,
        z: u16,
        id: u16,
        receiver37: Option<u64>,
    ) -> Option<(&Zone, usize)> {
        let zone = self.zone(x, y, z)?;
        let idx = zone.get_obj(x, z, id, receiver37)?;
        Some((zone, idx))
    }

    /// Returns the ground object matching `id` at the given tile.
    ///
    /// # Arguments
    ///
    /// * `x` -- The tile x coordinate.
    /// * `y` -- The level (height plane).
    /// * `z` -- The tile z coordinate.
    /// * `id` -- The object type id to match.
    /// * `receiver37` -- Base-37 username to match receiver-only objects against,
    ///   or `None` to consider only objects visible to everyone.
    ///
    /// # Returns
    ///
    /// `Some(&Obj)`, or `None` if the zone is not loaded or holds no such object
    /// visible to this receiver.
    #[inline]
    pub fn obj_at(&self, x: u16, y: u8, z: u16, id: u16, receiver37: Option<u64>) -> Option<&Obj> {
        let (zone, idx) = self.find_obj(x, y, z, id, receiver37)?;
        Some(&zone.objs[idx])
    }
}
