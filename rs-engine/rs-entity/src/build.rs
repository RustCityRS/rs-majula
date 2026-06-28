use rs_grid::{CoordGrid, ZoneCoordGrid};

/// Maximum number of players supported by the engine.
pub const MAX_PLAYERS: usize = 2048;
/// Maximum number of NPCs supported by the engine.
#[cfg(before_254)]
pub const MAX_NPCS: usize = 8192;
#[cfg(since_254)]
pub const MAX_NPCS: usize = 16384;

/// A compact set of entity IDs backed by a bit vector for O(1) membership checks
/// and an ordered ID list for iteration.
///
/// Uses unsafe pointer arithmetic for performance-critical `contains`, `insert`,
/// and `remove` operations on the bit vector.
#[allow(clippy::len_without_is_empty)]
pub struct IdBitSet {
    bits: Vec<u32>,
    ids: Vec<u16>,
}

impl IdBitSet {
    /// Creates a new `IdBitSet` with a bit vector sized for `len` IDs and a pre-allocated
    /// ID list of the given `capacity`.
    ///
    /// # Arguments
    /// * `len` - The maximum ID value this set can hold (rounded up to 32-bit words).
    /// * `capacity` - The initial capacity for the ordered ID list.
    #[inline]
    pub fn new(len: usize, capacity: usize) -> IdBitSet {
        IdBitSet {
            bits: vec![0; len >> 5],
            ids: Vec::with_capacity(capacity),
        }
    }

    /// Returns `true` if the given ID is present in the set. O(1) via bit check.
    ///
    /// # Arguments
    /// * `id` - The entity ID to test.
    #[inline]
    pub fn contains(&self, id: u16) -> bool {
        unsafe { *self.bits.as_ptr().add((id >> 5) as usize) & (1 << (id & 0x1F)) != 0 }
    }

    /// Inserts an ID into the set if not already present.
    ///
    /// # Arguments
    /// * `id` - The entity ID to add.
    ///
    /// # Side Effects
    /// * Sets the corresponding bit and appends the ID to the ordered list.
    #[inline]
    pub fn insert(&mut self, id: u16) {
        if self.contains(id) {
            return;
        }
        unsafe { *self.bits.as_mut_ptr().add((id >> 5) as usize) |= 1 << (id & 0x1F) };
        self.ids.push(id);
    }

    /// Removes an ID from the set if present.
    ///
    /// # Arguments
    /// * `id` - The entity ID to remove.
    ///
    /// # Side Effects
    /// * Clears the corresponding bit and removes the ID from the ordered list.
    #[inline]
    pub fn remove(&mut self, id: u16) {
        if !self.contains(id) {
            return;
        }
        unsafe { *self.bits.as_mut_ptr().add((id >> 5) as usize) &= !(1 << (id & 0x1F)) };
        if let Some(index) = self.ids.iter().position(|&x| x == id) {
            self.ids.remove(index);
        }
    }

    /// Clears only the bit for an ID without touching the ordered ID list.
    ///
    /// This is an O(1) operation intended for use when the caller has swapped
    /// out the `ids` vec and will reconcile it later via [`retain_bits`](Self::retain_bits).
    ///
    /// # Arguments
    /// * `id` - The entity ID whose bit to clear.
    #[inline]
    pub fn remove_bit(&mut self, id: u16) {
        unsafe { *self.bits.as_mut_ptr().add((id >> 5) as usize) &= !(1 << (id & 0x1F)) };
    }

    /// Removes entries from the ordered ID list whose bits have been cleared.
    ///
    /// Call this after one or more [`remove_bit`](Self::remove_bit) calls to
    /// bring the `ids` list back in sync with the bit vector.
    #[inline]
    pub fn retain_bits(&mut self) {
        let bits = self.bits.as_ptr();
        self.ids
            .retain(|&id| unsafe { *bits.add((id >> 5) as usize) & (1 << (id & 0x1F)) != 0 });
    }

    /// Swaps the internal ordered ID list with the provided vec.
    ///
    /// This is a pointer swap (no copy). Use this to temporarily take ownership
    /// of the ID list for iteration while the set's bit operations remain valid.
    #[inline]
    pub fn swap_ids(&mut self, other: &mut Vec<u16>) {
        std::mem::swap(&mut self.ids, other);
    }

    /// Returns the number of IDs currently in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Returns a slice of all IDs in insertion order for iteration.
    #[inline]
    pub fn iter(&self) -> &[u16] {
        &self.ids
    }

    /// Removes all IDs from the set by zeroing the bit vector and clearing the ID list.
    #[inline]
    pub fn clear(&mut self) {
        self.bits.fill(0);
        self.ids.clear();
    }
}

/// Tracks the player's viewport and the entities/zones visible to them.
///
/// The build area determines which map zones need to be sent to the client, which
/// players and NPCs are in range, and manages dynamic view distance that shrinks
/// when too many players are nearby and grows when player density decreases.
pub struct BuildArea {
    pub loaded_zones: Vec<ZoneCoordGrid>,
    pub active_zones: Vec<ZoneCoordGrid>,
    pub mapsquares: Vec<u16>,
    pub origin: CoordGrid,
    pub players: IdBitSet,
    pub npcs: IdBitSet,
    pub appearances: Box<[u32; MAX_PLAYERS]>,
    pub force_view_distance: bool,
    pub view_distance: u8,
    pub last_resize: u32,
    pub nearby_players: Vec<u16>,
    pub nearby_npcs: Vec<u16>,
}

impl BuildArea {
    /// Number of ticks between view distance growth attempts.
    pub const INTERVAL: u8 = 10;
    /// Maximum preferred number of tracked players before shrinking view distance.
    pub const PREFERRED_PLAYERS: u8 = 250;
    /// Maximum preferred number of tracked NPCs.
    pub const PREFERRED_NPCS: u8 = 255;
    /// The maximum and default view distance in zones.
    pub const PREFERRED_VIEW_DISTANCE: u8 = 15;

    /// Creates a new `BuildArea` with empty zone lists, default view distance,
    /// and pre-allocated ID sets.
    pub fn new() -> BuildArea {
        BuildArea {
            loaded_zones: Vec::with_capacity(7 * 7),
            active_zones: Vec::with_capacity(7 * 7),
            mapsquares: Vec::with_capacity(3 * 3),
            origin: CoordGrid::new(0, 0, 0),
            players: IdBitSet::new(MAX_PLAYERS, BuildArea::PREFERRED_PLAYERS as usize),
            npcs: IdBitSet::new(MAX_NPCS, BuildArea::PREFERRED_NPCS as usize),
            appearances: Box::new([0; MAX_PLAYERS]),
            force_view_distance: false,
            view_distance: BuildArea::PREFERRED_VIEW_DISTANCE,
            last_resize: 0,
            nearby_players: Vec::with_capacity(BuildArea::PREFERRED_PLAYERS as usize),
            nearby_npcs: Vec::with_capacity(BuildArea::PREFERRED_NPCS as usize),
        }
    }

    /// Clears all tracked state in the build area.
    ///
    /// If `reconnecting` is `true`, the clear is skipped entirely so the client retains
    /// its current viewport data across a reconnection.
    ///
    /// # Arguments
    /// * `reconnecting` - Whether the player is reconnecting to an existing session.
    ///
    /// # Side Effects
    /// * Clears `loaded_zones`, `active_zones`, `mapsquares`, `players`, `npcs`, and `appearances`.
    pub fn clear(&mut self, reconnecting: bool) {
        if reconnecting {
            return;
        }
        self.loaded_zones.clear();
        self.active_zones.clear();
        self.mapsquares.clear();
        self.players.clear();
        self.npcs.clear();
        self.appearances.fill(0);
    }

    /// Rebuilds the active zone list centered on the player's current coordinate.
    ///
    /// Computes a 7x7 zone grid centered on the player's zone, clipped to the bounds
    /// of the current build area origin (13x13 zone grid), and populates `active_zones`.
    ///
    /// # Arguments
    /// * `coord` - The player's current coordinate.
    ///
    /// # Side Effects
    /// * Clears and repopulates `self.active_zones`.
    pub fn rebuild_zones(&mut self, coord: CoordGrid) {
        self.active_zones.clear();

        let center_x = coord.zone_x();
        let center_z = coord.zone_z();

        let origin_x = self.origin.zone_x();
        let origin_z = self.origin.zone_z();

        let left_x = origin_x.saturating_sub(6);
        let right_x = origin_x.saturating_add(6);
        let top_z = origin_z.saturating_add(6);
        let bottom_z = origin_z.saturating_sub(6);

        for x in center_x.saturating_sub(3)..=center_x.saturating_add(3) {
            for z in center_z.saturating_sub(3)..=center_z.saturating_add(3) {
                if x < left_x || x > right_x || z > top_z || z < bottom_z {
                    continue;
                }
                self.active_zones
                    .push(ZoneCoordGrid::new(x << 3, coord.y(), z << 3));
            }
        }
    }

    /// Performs a full build area rebuild centered on the given coordinate.
    ///
    /// Recomputes the mapsquare list for the 13x13 zone grid, sets the build area
    /// origin to the given coordinate, and clears the loaded zone list so all zones
    /// are re-sent to the client.
    ///
    /// # Arguments
    /// * `coord` - The player's current coordinate to center the build area on.
    ///
    /// # Side Effects
    /// * Clears and repopulates `self.mapsquares`.
    /// * Sets `self.origin`.
    /// * Clears `self.loaded_zones`.
    pub fn rebuild_normal(&mut self, coord: &CoordGrid) {
        let zone_x = coord.zone_x();
        let zone_z = coord.zone_z();

        self.mapsquares.clear();
        let min_x = zone_x.saturating_sub(6);
        let max_x = zone_x.saturating_add(6);
        let min_z = zone_z.saturating_sub(6);
        let max_z = zone_z.saturating_add(6);

        for x in min_x..=max_x {
            let mx = CoordGrid::mapsquare(x << 3);
            for z in min_z..=max_z {
                let mz = CoordGrid::mapsquare(z << 3);
                let key = (mx << 8) | mz;
                if !self.mapsquares.contains(&key) {
                    self.mapsquares.push(key);
                }
            }
        }

        self.origin = CoordGrid::from(coord.packed());
        self.loaded_zones.clear();
    }

    /// Returns `true` if the player has moved far enough from the build area origin
    /// to require a full rebuild.
    ///
    /// A rebuild is needed when the player's zone coordinate differs from the origin
    /// by more than 4 zones in either axis, which means they are approaching the edge
    /// of the currently loaded 13x13 zone grid.
    ///
    /// # Arguments
    /// * `coord` - The player's current coordinate.
    pub fn needs_rebuild(&self, coord: &CoordGrid) -> bool {
        let cx = coord.zone_x() as i32;
        let cz = coord.zone_z() as i32;
        let ox = self.origin.zone_x() as i32;
        let oz = self.origin.zone_z() as i32;
        (cx - ox).abs() > 4 || (cz - oz).abs() > 4
    }

    /// Dynamically adjusts the view distance based on nearby player density.
    ///
    /// If `force_view_distance` is set, does nothing. Otherwise:
    /// - If there are >= `PREFERRED_PLAYERS` tracked, shrinks `view_distance` by 1
    ///   (minimum 1) and resets the resize counter.
    /// - Otherwise, increments the resize counter and grows `view_distance` by 1
    ///   (up to `PREFERRED_VIEW_DISTANCE`) every `INTERVAL` ticks.
    ///
    /// # Side Effects
    /// * May modify `self.view_distance` and `self.last_resize`.
    #[inline]
    pub fn resize(&mut self) {
        if self.force_view_distance {
            return;
        }

        if self.players.len() >= BuildArea::PREFERRED_PLAYERS as usize {
            if self.view_distance > 1 {
                self.view_distance -= 1;
            }
            self.last_resize = 0;
            return;
        }

        self.last_resize += 1;
        if self.last_resize >= BuildArea::INTERVAL as u32 {
            if self.view_distance < BuildArea::PREFERRED_VIEW_DISTANCE {
                self.view_distance += 1;
            } else {
                self.last_resize = 0;
            }
        }
    }

    /// Clears the tracked NPC set, forcing all NPCs to be re-evaluated for visibility
    /// on the next NPC info cycle.
    #[inline]
    pub fn rebuild_npcs(&mut self) {
        self.npcs.clear();
    }

    /// Returns `true` if the stored appearance clock for the given player matches
    /// the provided clock, meaning the client already has the latest appearance data.
    ///
    /// # Arguments
    /// * `pid` - The player index.
    /// * `clock` - The current appearance version clock.
    #[inline]
    pub const fn has_appearance(&self, pid: u16, clock: u32) -> bool {
        unsafe { *self.appearances.as_ptr().add(pid as usize) == clock }
    }

    /// Saves the appearance version clock for the given player, indicating the client
    /// now has up-to-date appearance data for that player.
    ///
    /// # Arguments
    /// * `pid` - The player index.
    /// * `clock` - The appearance version clock to store.
    #[inline]
    pub fn save_appearance(&mut self, pid: u16, clock: u32) {
        unsafe { *self.appearances.as_mut_ptr().add(pid as usize) = clock }
    }
}
