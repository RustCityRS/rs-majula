/// A packed 30-bit coordinate representing a tile position in the game world.
///
/// `CoordGrid` encodes three components into a single `u32`:
/// - **Z** (bits 0..13): 14-bit east-west position (range 0..16383).
/// - **X** (bits 14..27): 14-bit north-south position (range 0..16383).
/// - **Y** (bits 28..29): 2-bit vertical level/plane (range 0..3).
///
/// ## Bit layout
///
/// ```text
/// ((z & 0x3FFF)) | ((x & 0x3FFF) << 14) | ((y & 0x3) << 28)
///
/// 31 30 29 28 | 27 .............. 14 | 13 .............. 0
///  0  0  y  y |  x  x  x  x ... x  x |  z  z  z  z ... z  z
/// ```
///
/// This compact representation enables efficient storage, hashing, and
/// comparison of tile coordinates. It is the primary coordinate type used
/// throughout the game engine for entity positions, pathfinding, zone
/// management, and map lookups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CoordGrid(u32);

impl CoordGrid {
    /// Creates a new `CoordGrid` by packing the given X, Y, and Z components
    /// into a single `u32`.
    ///
    /// Each component is masked to its valid bit width before packing, so
    /// values exceeding the range silently wrap (X and Z are masked to 14 bits,
    /// Y to 2 bits).
    ///
    /// # Arguments
    ///
    /// * `x` - The X (north-south) tile coordinate. Only the low 14 bits are used (0..16383).
    /// * `y` - The Y level/plane. Only the low 2 bits are used (0..3).
    /// * `z` - The Z (east-west) tile coordinate. Only the low 14 bits are used (0..16383).
    ///
    /// # Returns
    ///
    /// A `CoordGrid` with the components packed as `z | (x << 14) | (y << 28)`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Used extensively throughout the engine for constructing
    /// tile positions (player movement, NPC spawning, map loading, scripting opcodes, etc.).
    ///
    /// **Calls:** Nothing.
    #[inline(always)]
    pub const fn new(x: u16, y: u8, z: u16) -> Self {
        CoordGrid(
            ((z & 0x3FFF) as u32) | (((x & 0x3FFF) as u32) << 14) | (((y & 0x3) as u32) << 28),
        )
    }

    /// Wraps a raw pre-packed `u32` value as a `CoordGrid`.
    ///
    /// No validation or masking is performed -- the caller is responsible for
    /// ensuring the value follows the expected bit layout. This is typically
    /// used when reading coordinates that were previously packed or received
    /// from network/data sources.
    ///
    /// # Arguments
    ///
    /// * `packed` - A `u32` already in the `CoordGrid` bit layout.
    ///
    /// # Returns
    ///
    /// A `CoordGrid` wrapping the given value directly.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Used extensively throughout the engine when deserializing
    /// coordinates from packed integer form (map data, network packets, cache lookups).
    ///
    /// **Calls:** Nothing.
    #[inline(always)]
    pub const fn from(packed: u32) -> Self {
        CoordGrid(packed)
    }

    /// Returns the raw packed `u32` representation of this coordinate.
    ///
    /// The returned value encodes Z in bits 0..13, X in bits 14..27, and Y in
    /// bits 28..29. This is useful for serialization, hashing, or passing the
    /// coordinate through integer channels (e.g., network packets, script variables).
    ///
    /// # Returns
    ///
    /// The inner `u32` with the bit layout `z | (x << 14) | (y << 28)`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Used extensively throughout the engine for serializing
    /// coordinates, map key lookups, and network encoding.
    ///
    /// **Calls:** Nothing.
    #[inline(always)]
    pub const fn packed(&self) -> u32 {
        self.0
    }

    /// Unpacks this coordinate into its three individual components.
    ///
    /// This is a convenience method equivalent to calling [`x()`](Self::x),
    /// [`y()`](Self::y), and [`z()`](Self::z) individually.
    ///
    /// # Returns
    ///
    /// A tuple `(x, y, z)` with the unpacked tile coordinates.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Used extensively throughout the engine when all three
    /// components are needed at once (e.g., map indexing, coordinate transforms).
    ///
    /// **Calls:** [`x()`](Self::x), [`y()`](Self::y), [`z()`](Self::z).
    #[inline(always)]
    pub const fn index(&self) -> (u16, u8, u16) {
        (self.x(), self.y(), self.z())
    }

    /// Extracts the X (north-south) tile coordinate from bits 14..27.
    ///
    /// # Returns
    ///
    /// The 14-bit X component as a `u16` (range 0..16383).
    ///
    /// # Call Stack
    ///
    /// **Called by:** Used extensively throughout the engine for position checks,
    /// distance calculations, zone/mapsquare derivation, and rendering.
    ///
    /// **Calls:** Nothing.
    #[inline(always)]
    pub const fn x(&self) -> u16 {
        ((self.0 >> 14) & 0x3FFF) as u16
    }

    /// Extracts the Y level/plane from bits 28..29.
    ///
    /// The Y component represents the vertical plane or floor of the game
    /// world. With only 2 bits, valid values are 0 through 3.
    ///
    /// # Returns
    ///
    /// The 2-bit Y component as a `u8` (range 0..3).
    ///
    /// # Call Stack
    ///
    /// **Called by:** Used extensively throughout the engine for level-based
    /// filtering, collision plane selection, and multi-floor logic.
    ///
    /// **Calls:** Nothing.
    #[inline(always)]
    pub const fn y(&self) -> u8 {
        ((self.0 >> 28) & 0x3) as u8
    }

    /// Extracts the Z (east-west) tile coordinate from bits 0..13.
    ///
    /// # Returns
    ///
    /// The 14-bit Z component as a `u16` (range 0..16383).
    ///
    /// # Call Stack
    ///
    /// **Called by:** Used extensively throughout the engine for position checks,
    /// distance calculations, zone/mapsquare derivation, and rendering.
    ///
    /// **Calls:** Nothing.
    #[inline(always)]
    pub const fn z(&self) -> u16 {
        (self.0 & 0x3FFF) as u16
    }

    /// Converts a tile position to its zone index by dividing by 8 (`pos >> 3`).
    ///
    /// Zones are 8x8 tile regions used for spatial partitioning. This static
    /// method works on a single axis coordinate.
    ///
    /// # Arguments
    ///
    /// * `pos` - A tile coordinate on a single axis (X or Z).
    ///
    /// # Returns
    ///
    /// The zone index for the given tile position.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`zone_center()`](Self::zone_center), and directly by
    /// engine systems performing zone-level spatial queries.
    ///
    /// **Calls:** Nothing.
    #[inline(always)]
    pub const fn zone(pos: u16) -> u16 {
        pos >> 3
    }

    /// Returns the zone index for this coordinate's X component.
    ///
    /// Equivalent to `self.x() >> 3`.
    ///
    /// # Returns
    ///
    /// The zone index along the X axis.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Zone change detection, build area management, and entity
    /// visibility systems throughout the engine.
    ///
    /// **Calls:** [`x()`](Self::x).
    #[inline(always)]
    pub const fn zone_x(&self) -> u16 {
        self.x() >> 3
    }

    /// Returns the zone index for this coordinate's Z component.
    ///
    /// Equivalent to `self.z() >> 3`.
    ///
    /// # Returns
    ///
    /// The zone index along the Z axis.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Zone change detection, build area management, and entity
    /// visibility systems throughout the engine.
    ///
    /// **Calls:** [`z()`](Self::z).
    #[inline(always)]
    pub const fn zone_z(&self) -> u16 {
        self.z() >> 3
    }

    /// Computes the center zone index for a tile position.
    ///
    /// The center zone index is the zone index offset by -6, which positions
    /// it at the center of the player's visible build area (a 13x13 zone grid).
    /// This is used to determine the origin of the build area that the client
    /// renders around the player.
    ///
    /// # Arguments
    ///
    /// * `pos` - A tile coordinate on a single axis (X or Z).
    ///
    /// # Returns
    ///
    /// The center zone index: `zone(pos) - 6`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`zone_origin()`](Self::zone_origin), and directly by
    /// build area and client update systems.
    ///
    /// **Calls:** [`zone()`](Self::zone).
    #[inline(always)]
    pub const fn zone_center(pos: u16) -> u16 {
        Self::zone(pos) - 6
    }

    /// Returns the center zone index for this coordinate's X component.
    ///
    /// Equivalent to `self.zone_x() - 6`.
    ///
    /// # Returns
    ///
    /// The center zone index along the X axis.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Build area initialization and zone origin calculations.
    ///
    /// **Calls:** [`zone_x()`](Self::zone_x).
    #[inline(always)]
    pub const fn zone_center_x(&self) -> u16 {
        self.zone_x() - 6
    }

    /// Returns the center zone index for this coordinate's Z component.
    ///
    /// Equivalent to `self.zone_z() - 6`.
    ///
    /// # Returns
    ///
    /// The center zone index along the Z axis.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Build area initialization and zone origin calculations.
    ///
    /// **Calls:** [`zone_z()`](Self::zone_z).
    #[inline(always)]
    pub const fn zone_center_z(&self) -> u16 {
        self.zone_z() - 6
    }

    /// Computes the origin tile coordinate for the center zone of a tile position.
    ///
    /// This converts the center zone index back to a tile coordinate by
    /// shifting left by 3 (multiplying by 8). The result is the south-west
    /// tile of the build area origin zone.
    ///
    /// # Arguments
    ///
    /// * `pos` - A tile coordinate on a single axis (X or Z).
    ///
    /// # Returns
    ///
    /// The origin tile coordinate: `zone_center(pos) << 3`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Build area management and client map rebuild triggers.
    ///
    /// **Calls:** [`zone_center()`](Self::zone_center).
    #[inline(always)]
    pub const fn zone_origin(pos: u16) -> u16 {
        Self::zone_center(pos) << 3
    }

    /// Returns the origin tile coordinate along X for this coordinate's center zone.
    ///
    /// Equivalent to `self.zone_center_x() << 3`.
    ///
    /// # Returns
    ///
    /// The X tile coordinate at the origin of the center zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Build area management and client map rebuild triggers.
    ///
    /// **Calls:** [`zone_center_x()`](Self::zone_center_x).
    #[inline(always)]
    pub const fn zone_origin_x(&self) -> u16 {
        self.zone_center_x() << 3
    }

    /// Returns the origin tile coordinate along Z for this coordinate's center zone.
    ///
    /// Equivalent to `self.zone_center_z() << 3`.
    ///
    /// # Returns
    ///
    /// The Z tile coordinate at the origin of the center zone.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Build area management and client map rebuild triggers.
    ///
    /// **Calls:** [`zone_center_z()`](Self::zone_center_z).
    #[inline(always)]
    pub const fn zone_origin_z(&self) -> u16 {
        self.zone_center_z() << 3
    }

    /// Converts a tile position to its mapsquare index by dividing by 64 (`pos >> 6`).
    ///
    /// Mapsquares are 64x64 tile regions that correspond to individual map
    /// files in the game cache. Each mapsquare contains 8x8 zones.
    ///
    /// # Arguments
    ///
    /// * `pos` - A tile coordinate on a single axis (X or Z).
    ///
    /// # Returns
    ///
    /// The mapsquare index for the given tile position.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Map loading, cache lookups, and world partitioning systems.
    ///
    /// **Calls:** Nothing.
    #[inline(always)]
    pub const fn mapsquare(pos: u16) -> u16 {
        pos >> 6
    }

    /// Returns the mapsquare index for this coordinate's X component.
    ///
    /// Equivalent to `self.x() >> 6`.
    ///
    /// # Returns
    ///
    /// The mapsquare index along the X axis.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Map loading, cache lookups, and world partitioning systems.
    ///
    /// **Calls:** [`x()`](Self::x).
    #[inline(always)]
    pub const fn mapsquare_x(&self) -> u16 {
        self.x() >> 6
    }

    /// Returns the mapsquare index for this coordinate's Z component.
    ///
    /// Equivalent to `self.z() >> 6`.
    ///
    /// # Returns
    ///
    /// The mapsquare index along the Z axis.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Map loading, cache lookups, and world partitioning systems.
    ///
    /// **Calls:** [`z()`](Self::z).
    #[inline(always)]
    pub const fn mapsquare_z(&self) -> u16 {
        self.z() >> 6
    }

    /// Computes a fine-grained position for entity info update packets.
    ///
    /// The fine position places the entity's anchor at the center of its
    /// occupied area by computing `pos * 2 + size`. This is used when
    /// sending position updates to the client at sub-tile granularity
    /// for entities that occupy more than one tile.
    ///
    /// # Arguments
    ///
    /// * `pos` - A tile coordinate on a single axis (X or Z).
    /// * `size` - The entity's size (width or length) in tiles.
    ///
    /// # Returns
    ///
    /// The fine-grained position value for the info update.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Player and NPC info encoding systems when writing
    /// position data to client update packets.
    ///
    /// **Calls:** Nothing.
    #[inline(always)]
    pub const fn fine(pos: u16, size: u8) -> u16 {
        pos * 2 + size as u16
    }

    /// Packs two zone-local offsets (each `0..=7`) into a single protocol byte:
    /// the upper nibble holds `x`, the lower nibble holds `z`.
    ///
    /// This is the single source of truth for the zone-coord byte layout --
    /// [`packed_zone_coord`](Self::packed_zone_coord) and the `Loc`/`Obj`
    /// `packed_zone_coord` helpers all delegate here.
    #[inline(always)]
    pub const fn packed_zone_coord(x: u16, z: u16) -> u8 {
        (((x & 0x7) << 4) as u8) | ((z & 0x7) as u8)
    }

    /// Returns `true` if the zone-local offsets (`local_x`, `local_z`, each
    /// `0..=7`) match the low 3 bits of the world coordinate (`x`, `z`) -- i.e.
    /// they reference the same tile within a single zone.
    ///
    /// The single source of truth for `Loc::is_at` / `Obj::is_at`.
    #[inline(always)]
    pub const fn local_eq(local_x: u8, local_z: u8, x: u16, z: u16) -> bool {
        local_x == (x & 0x7) as u8 && local_z == (z & 0x7) as u8
    }

    /// Checks whether another coordinate is within a given Chebyshev distance
    /// on the X-Z plane.
    ///
    /// This is equivalent to checking that the absolute difference on both
    /// the X and Z axes is at most `distance`. The Y (level) component is
    /// **not** considered. This check uses a "box" distance (L-infinity norm),
    /// not Euclidean distance.
    ///
    /// # Arguments
    ///
    /// * `other` - The coordinate to compare against.
    /// * `distance` - The maximum allowed Chebyshev distance (inclusive).
    ///
    /// # Returns
    ///
    /// `true` if both `|self.x() - other.x()| <= distance` and
    /// `|self.z() - other.z()| <= distance`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** NPC/player visibility checks, interaction range
    /// validation, and entity search filtering throughout the engine.
    ///
    /// **Calls:** [`x()`](Self::x), [`z()`](Self::z).
    #[inline(always)]
    pub const fn in_distance(&self, other: CoordGrid, distance: u8) -> bool {
        !((self.x() as i32 - other.x() as i32).abs() > distance as i32
            || (self.z() as i32 - other.z() as i32).abs() > distance as i32)
    }

    /// Computes the Chebyshev (L-infinity) distance to another coordinate
    /// on the X-Z plane.
    ///
    /// Returns `max(|dx|, |dz|)` where `dx = self.x() - other.x()` and
    /// `dz = self.z() - other.z()`. The Y (level) component is **not**
    /// considered. This metric corresponds to king-move distance on a grid.
    ///
    /// # Arguments
    ///
    /// * `other` - The coordinate to measure distance to.
    ///
    /// # Returns
    ///
    /// The Chebyshev distance as a non-negative `i32`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Interaction distance checks, combat range validation,
    /// and pathfinding heuristics throughout the engine.
    ///
    /// **Calls:** [`x()`](Self::x), [`z()`](Self::z).
    #[inline(always)]
    pub fn distance(&self, other: CoordGrid) -> i32 {
        let (dx, dz) = self.abs_deltas(other);
        dx.max(dz)
    }

    /// Returns the absolute X and Z deltas to another coordinate on the X-Z
    /// plane as `(|dx|, |dz|)`. The Y (level) component is not considered.
    #[inline(always)]
    const fn abs_deltas(&self, other: CoordGrid) -> (i32, i32) {
        let dx = (self.x() as i32 - other.x() as i32).abs();
        let dz = (self.z() as i32 - other.z() as i32).abs();
        (dx, dz)
    }

    /// Computes the Chebyshev distance between two axis-aligned bounding
    /// rectangles (AABBs).
    ///
    /// Each rectangle is defined by its south-west origin `(src_x, src_z)` and
    /// dimensions `(src_w, src_l)`. The distance is measured between the
    /// closest points on each rectangle's perimeter. If the rectangles overlap,
    /// the distance is 0.
    ///
    /// # Arguments
    ///
    /// * `src_x` - X origin of the source rectangle.
    /// * `src_z` - Z origin of the source rectangle.
    /// * `src_w` - Width of the source rectangle (X extent).
    /// * `src_l` - Length of the source rectangle (Z extent).
    /// * `dst_x` - X origin of the destination rectangle.
    /// * `dst_z` - Z origin of the destination rectangle.
    /// * `dst_w` - Width of the destination rectangle (X extent).
    /// * `dst_l` - Length of the destination rectangle (Z extent).
    ///
    /// # Returns
    ///
    /// The Chebyshev distance between the two closest points of the
    /// rectangles, as a non-negative `i32`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Multi-tile entity interaction checks (e.g., determining
    /// if a player can reach a large NPC or game object).
    ///
    /// **Calls:** [`closest()`](Self::closest).
    #[inline(always)]
    pub fn distance_to(
        src_x: i32,
        src_z: i32,
        src_w: i32,
        src_l: i32,
        dst_x: i32,
        dst_z: i32,
        dst_w: i32,
        dst_l: i32,
    ) -> i32 {
        let (p1x, p1z) = Self::closest(src_x, src_z, src_w, src_l, dst_x, dst_z);
        let (p2x, p2z) = Self::closest(dst_x, dst_z, dst_w, dst_l, src_x, src_z);
        (p1x - p2x).abs().max((p1z - p2z).abs())
    }

    /// Finds the closest point on a source rectangle to a given destination point.
    ///
    /// The result is the point on (or within) the rectangle defined by origin
    /// `(src_x, src_z)` and dimensions `(src_w, src_l)` that is nearest to
    /// `(dst_x, dst_z)`. Each axis is independently clamped to the rectangle's
    /// extent `[src, src + size - 1]`.
    ///
    /// # Arguments
    ///
    /// * `src_x` - X origin of the source rectangle.
    /// * `src_z` - Z origin of the source rectangle.
    /// * `src_w` - Width of the source rectangle (X extent).
    /// * `src_l` - Length of the source rectangle (Z extent).
    /// * `dst_x` - X coordinate of the destination point.
    /// * `dst_z` - Z coordinate of the destination point.
    ///
    /// # Returns
    ///
    /// A tuple `(cx, cz)` representing the closest point on the source
    /// rectangle to the destination point.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`distance_to()`](Self::distance_to).
    ///
    /// **Calls:** Nothing.
    #[inline(always)]
    fn closest(
        src_x: i32,
        src_z: i32,
        src_w: i32,
        src_l: i32,
        dst_x: i32,
        dst_z: i32,
    ) -> (i32, i32) {
        let occ_x = src_x + src_w - 1;
        let occ_z = src_z + src_l - 1;
        let cx = if dst_x <= src_x {
            src_x
        } else if dst_x >= occ_x {
            occ_x
        } else {
            dst_x
        };
        let cz = if dst_z <= src_z {
            src_z
        } else if dst_z >= occ_z {
            occ_z
        } else {
            dst_z
        };
        (cx, cz)
    }

    /// Computes the squared Euclidean distance to another coordinate on the
    /// X-Z plane.
    ///
    /// Returns `dx * dx + dz * dz` where `dx` and `dz` are the absolute
    /// differences on each axis. The Y (level) component is **not** considered.
    /// Using the squared distance avoids a square root operation, which is
    /// sufficient for distance comparisons and ordering.
    ///
    /// # Arguments
    ///
    /// * `other` - The coordinate to measure distance to.
    ///
    /// # Returns
    ///
    /// The squared Euclidean distance as a non-negative `i32`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Entity sorting by proximity (e.g., finding the nearest
    /// NPC or player) where relative ordering matters more than exact distance.
    ///
    /// **Calls:** [`x()`](Self::x), [`z()`](Self::z).
    #[inline(always)]
    pub fn euclidean_squared_distance(&self, other: CoordGrid) -> i32 {
        let (dx, dz) = self.abs_deltas(other);
        dx * dx + dz * dz
    }

    /// Checks whether this coordinate falls within the Wilderness region.
    ///
    /// The Wilderness is defined by two rectangular bounds on the X-Z plane
    /// (covering both the overworld and mirrored regions):
    /// - Overworld: X in `[2944, 3392)`, Z in `[3520, 6400)`
    /// - Mirrored:  X in `[2944, 3392)`, Z in `[9920, 12800)`
    ///
    /// The Y (level) component is **not** considered.
    ///
    /// # Returns
    ///
    /// `true` if this coordinate is inside either Wilderness rectangle.
    ///
    /// # Call Stack
    ///
    /// **Called by:** PvP combat eligibility, wilderness-specific content
    /// gating, and multi-combat zone checks.
    ///
    /// **Calls:** [`x()`](Self::x), [`z()`](Self::z).
    #[inline(always)]
    pub const fn is_in_wilderness(&self) -> bool {
        let x = self.x();
        let z = self.z();
        (x >= 2944 && x < 3392 && z >= 3520 && z < 6400)
            || (x >= 2944 && x < 3392 && z >= 9920 && z < 12800)
    }

    /// Tests whether two axis-aligned bounding rectangles (AABBs) overlap.
    ///
    /// Each rectangle is defined by its south-west origin and dimensions.
    /// The test uses strict inequality, so rectangles that merely share an
    /// edge (touching but not overlapping) return `false`.
    ///
    /// # Arguments
    ///
    /// * `src_x` - X origin of the first rectangle.
    /// * `src_z` - Z origin of the first rectangle.
    /// * `src_w` - Width of the first rectangle (X extent).
    /// * `src_h` - Height of the first rectangle (Z extent).
    /// * `dest_x` - X origin of the second rectangle.
    /// * `dest_z` - Z origin of the second rectangle.
    /// * `dest_w` - Width of the second rectangle (X extent).
    /// * `dest_h` - Height of the second rectangle (Z extent).
    ///
    /// # Returns
    ///
    /// `true` if the two rectangles have any area of overlap.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Collision detection, multi-tile entity overlap checks,
    /// and area-of-effect targeting systems.
    ///
    /// **Calls:** Nothing.
    pub fn intersects(
        src_x: u16,
        src_z: u16,
        src_w: u16,
        src_h: u16,
        dest_x: u16,
        dest_z: u16,
        dest_w: u16,
        dest_h: u16,
    ) -> bool {
        let src_right = src_x + src_w;
        let src_top = src_z + src_h;
        let dest_right = dest_x + dest_w;
        let dest_top = dest_z + dest_h;
        !(dest_x >= src_right || dest_right <= src_x || dest_z >= src_top || dest_top <= src_z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_accessors_basic() {
        let coord = CoordGrid::new(100, 2, 200);
        assert_eq!(coord.x(), 100);
        assert_eq!(coord.y(), 2);
        assert_eq!(coord.z(), 200);
    }

    #[test]
    fn index_returns_tuple() {
        let coord = CoordGrid::new(50, 1, 75);
        assert_eq!(coord.index(), (50, 1, 75));
    }

    #[test]
    fn packed_round_trip() {
        let coord = CoordGrid::new(3200, 3, 3200);
        let packed = coord.packed();
        let reconstructed = CoordGrid(packed);
        assert_eq!(reconstructed.x(), 3200);
        assert_eq!(reconstructed.y(), 3);
        assert_eq!(reconstructed.z(), 3200);
    }

    #[test]
    fn y_wraps_at_4() {
        let coord = CoordGrid::new(0, 3, 0);
        assert_eq!(coord.y(), 3);
        // y is 2 bits so 4 wraps to 0
        let coord2 = CoordGrid::new(0, 4, 0);
        assert_eq!(coord2.y(), 0);
    }

    #[test]
    fn x_z_max_14_bits() {
        let max_14 = 0x3FFF; // 16383
        let coord = CoordGrid::new(max_14, 3, max_14);
        assert_eq!(coord.x(), max_14);
        assert_eq!(coord.z(), max_14);
    }

    #[test]
    fn x_z_overflow_masked() {
        let coord = CoordGrid::new(0x4000, 0, 0x4000);
        assert_eq!(coord.x(), 0);
        assert_eq!(coord.z(), 0);
    }

    #[test]
    fn default_is_zero() {
        let coord = CoordGrid::default();
        assert_eq!(coord.0, 0);
        assert_eq!(coord.x(), 0);
        assert_eq!(coord.y(), 0);
        assert_eq!(coord.z(), 0);
    }

    #[test]
    fn zone_calculations() {
        let coord = CoordGrid::new(3200, 0, 3200);
        assert_eq!(coord.zone_x(), 3200 >> 3);
        assert_eq!(coord.zone_z(), 3200 >> 3);
        assert_eq!(CoordGrid::zone(3200), 400);
    }

    #[test]
    fn zone_center_calculations() {
        assert_eq!(CoordGrid::zone_center(3200), (3200 >> 3) - 6);
        let coord = CoordGrid::new(3200, 0, 3200);
        assert_eq!(coord.zone_center_x(), coord.zone_x() - 6);
        assert_eq!(coord.zone_center_z(), coord.zone_z() - 6);
    }

    #[test]
    fn zone_origin_calculations() {
        let coord = CoordGrid::new(3200, 0, 3200);
        let expected_x = ((3200u16 >> 3) - 6) << 3;
        let expected_z = ((3200u16 >> 3) - 6) << 3;
        assert_eq!(coord.zone_origin_x(), expected_x);
        assert_eq!(coord.zone_origin_z(), expected_z);
        assert_eq!(CoordGrid::zone_origin(3200), expected_x);
    }

    #[test]
    fn mapsquare_calculations() {
        let coord = CoordGrid::new(3200, 0, 3200);
        assert_eq!(coord.mapsquare_x(), 3200 >> 6);
        assert_eq!(coord.mapsquare_z(), 3200 >> 6);
        assert_eq!(CoordGrid::mapsquare(3200), 50);
    }

    #[test]
    fn fine_calculation() {
        assert_eq!(CoordGrid::fine(100, 1), 201);
        assert_eq!(CoordGrid::fine(0, 0), 0);
        assert_eq!(CoordGrid::fine(50, 2), 102);
    }

    #[test]
    fn distance_same_point() {
        let a = CoordGrid::new(100, 0, 100);
        assert_eq!(a.distance(a), 0);
    }

    #[test]
    fn distance_chebyshev() {
        let a = CoordGrid::new(100, 0, 100);
        let b = CoordGrid::new(103, 0, 105);
        assert_eq!(a.distance(b), 5); // max(3, 5) = 5
    }

    #[test]
    fn distance_symmetric() {
        let a = CoordGrid::new(100, 0, 100);
        let b = CoordGrid::new(110, 0, 105);
        assert_eq!(a.distance(b), b.distance(a));
    }

    #[test]
    fn in_distance_exact_boundary() {
        let a = CoordGrid::new(100, 0, 100);
        let b = CoordGrid::new(105, 0, 100);
        assert!(a.in_distance(b, 5));
        assert!(!a.in_distance(b, 4));
    }

    #[test]
    fn in_distance_diagonal() {
        let a = CoordGrid::new(100, 0, 100);
        let b = CoordGrid::new(103, 0, 103);
        assert!(a.in_distance(b, 3));
        assert!(!a.in_distance(b, 2));
    }

    #[test]
    fn in_distance_same_coord() {
        let a = CoordGrid::new(50, 1, 50);
        assert!(a.in_distance(a, 0));
    }

    #[test]
    fn in_distance_ignores_y() {
        let a = CoordGrid::new(100, 0, 100);
        let b = CoordGrid::new(100, 3, 100);
        assert!(a.in_distance(b, 0));
    }

    #[test]
    fn intersects_overlapping() {
        assert!(CoordGrid::intersects(0, 0, 10, 10, 5, 5, 10, 10));
    }

    #[test]
    fn intersects_no_overlap() {
        assert!(!CoordGrid::intersects(0, 0, 5, 5, 10, 10, 5, 5));
    }

    #[test]
    fn intersects_touching_edge_no_overlap() {
        assert!(!CoordGrid::intersects(0, 0, 5, 5, 5, 0, 5, 5));
    }

    #[test]
    fn intersects_contained() {
        assert!(CoordGrid::intersects(0, 0, 20, 20, 5, 5, 5, 5));
    }

    #[test]
    fn intersects_same_rect() {
        assert!(CoordGrid::intersects(10, 10, 5, 5, 10, 10, 5, 5));
    }

    #[test]
    fn intersects_single_point() {
        assert!(CoordGrid::intersects(5, 5, 1, 1, 5, 5, 1, 1));
    }

    #[test]
    fn intersects_adjacent_no_overlap() {
        assert!(!CoordGrid::intersects(0, 0, 5, 5, 0, 5, 5, 5));
    }

    #[test]
    fn equality_and_hash() {
        let a = CoordGrid::new(100, 1, 200);
        let b = CoordGrid::new(100, 1, 200);
        let c = CoordGrid::new(100, 2, 200);
        assert_eq!(a, b);
        assert_ne!(a, c);

        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }

    #[allow(clippy::clone_on_copy)]
    #[test]
    fn copy_and_clone() {
        let a = CoordGrid::new(100, 1, 200);
        let b = a;
        let c = a.clone();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn various_coordinates() {
        for x in [0u16, 1, 64, 3200, 16383] {
            for y in [0u8, 1, 2, 3] {
                for z in [0u16, 1, 64, 3200, 16383] {
                    let coord = CoordGrid::new(x, y, z);
                    assert_eq!(coord.x(), x, "x mismatch for ({x},{y},{z})");
                    assert_eq!(coord.y(), y, "y mismatch for ({x},{y},{z})");
                    assert_eq!(coord.z(), z, "z mismatch for ({x},{y},{z})");
                }
            }
        }
    }

    // --- INZONE opcode pattern ---

    #[test]
    fn inzone_inside() {
        let sw = CoordGrid::new(3200, 0, 3200);
        let ne = CoordGrid::new(3210, 0, 3210);
        let test = CoordGrid::new(3205, 0, 3205);
        let ok = test.y() == sw.y()
            && test.x() >= sw.x()
            && test.x() <= ne.x()
            && test.z() >= sw.z()
            && test.z() <= ne.z();
        assert!(ok);
    }

    #[test]
    fn inzone_outside() {
        let sw = CoordGrid::new(3200, 0, 3200);
        let ne = CoordGrid::new(3210, 0, 3210);
        let test = CoordGrid::new(3215, 0, 3205);
        let ok = test.y() == sw.y()
            && test.x() >= sw.x()
            && test.x() <= ne.x()
            && test.z() >= sw.z()
            && test.z() <= ne.z();
        assert!(!ok);
    }

    #[test]
    fn inzone_wrong_level() {
        let sw = CoordGrid::new(3200, 0, 3200);
        let _ne = CoordGrid::new(3210, 0, 3210);
        let test = CoordGrid::new(3205, 1, 3205);
        let ok = test.y() == sw.y();
        assert!(!ok);
    }

    #[test]
    fn inzone_on_boundary() {
        let sw = CoordGrid::new(3200, 0, 3200);
        let ne = CoordGrid::new(3210, 0, 3210);
        // Test on sw corner
        let on_sw = sw.y() == sw.y()
            && sw.x() >= sw.x()
            && sw.x() <= ne.x()
            && sw.z() >= sw.z()
            && sw.z() <= ne.z();
        assert!(on_sw);
        // Test on ne corner
        let on_ne = ne.y() == sw.y()
            && ne.x() >= sw.x()
            && ne.x() <= ne.x()
            && ne.z() >= sw.z()
            && ne.z() <= ne.z();
        assert!(on_ne);
    }

    // --- MOVECOORD opcode pattern ---

    #[test]
    fn movecoord_positive_offset() {
        let base = CoordGrid::new(3200, 0, 3200);
        let dx = 5;
        let dy = 1;
        let dz = 10;
        let nc = CoordGrid::new(
            (base.x() as i32 + dx) as u16,
            (base.y() as i32 + dy).clamp(0, 3) as u8,
            (base.z() as i32 + dz) as u16,
        );
        assert_eq!(nc.x(), 3205);
        assert_eq!(nc.y(), 1);
        assert_eq!(nc.z(), 3210);
    }

    #[test]
    fn movecoord_negative_offset() {
        let base = CoordGrid::new(3200, 2, 3200);
        let dx = -10;
        let dy = -1;
        let dz = -5;
        let nc = CoordGrid::new(
            (base.x() as i32 + dx) as u16,
            (base.y() as i32 + dy).clamp(0, 3) as u8,
            (base.z() as i32 + dz) as u16,
        );
        assert_eq!(nc.x(), 3190);
        assert_eq!(nc.y(), 1);
        assert_eq!(nc.z(), 3195);
    }

    #[test]
    fn movecoord_y_clamp_above() {
        let base = CoordGrid::new(100, 3, 100);
        let dy = 5; // would exceed 3
        let nc_y = (base.y() as i32 + dy).clamp(0, 3) as u8;
        assert_eq!(nc_y, 3);
    }

    #[test]
    fn movecoord_y_clamp_below() {
        let base = CoordGrid::new(100, 0, 100);
        let dy = -5; // would go negative
        let nc_y = (base.y() as i32 + dy).clamp(0, 3) as u8;
        assert_eq!(nc_y, 0);
    }

    // --- Zone change detection pattern ---

    #[test]
    fn zone_change_detection_same_zone() {
        let prev = CoordGrid::new(3200, 0, 3200);
        let next = CoordGrid::new(3201, 0, 3201);
        let zone_changed = prev.zone_x() != next.zone_x()
            || prev.zone_z() != next.zone_z()
            || prev.y() != next.y();
        assert!(!zone_changed); // within same zone (8-tile granularity)
    }

    #[test]
    fn zone_change_detection_cross_zone() {
        let prev = CoordGrid::new(3199, 0, 3200); // zone_x = 399
        let next = CoordGrid::new(3200, 0, 3200); // zone_x = 400
        let zone_changed = prev.zone_x() != next.zone_x()
            || prev.zone_z() != next.zone_z()
            || prev.y() != next.y();
        assert!(zone_changed);
    }

    #[test]
    fn zone_change_detection_level_change() {
        let prev = CoordGrid::new(3200, 0, 3200);
        let next = CoordGrid::new(3200, 1, 3200);
        let zone_changed = prev.zone_x() != next.zone_x()
            || prev.zone_z() != next.zone_z()
            || prev.y() != next.y();
        assert!(zone_changed);
    }

    // --- Distance for NPC finding patterns ---

    #[test]
    fn distance_large_separation() {
        let a = CoordGrid::new(3200, 0, 3200);
        let b = CoordGrid::new(3300, 0, 3250);
        assert_eq!(a.distance(b), 100); // max(100, 50)
    }

    #[test]
    fn in_distance_npc_find_range() {
        let player = CoordGrid::new(3200, 0, 3200);
        let npc_close = CoordGrid::new(3205, 0, 3203);
        let npc_far = CoordGrid::new(3220, 0, 3200);
        assert!(player.in_distance(npc_close, 15));
        assert!(!player.in_distance(npc_far, 15));
    }

    #[test]
    fn distance_across_y_levels() {
        // Distance ignores y, only uses x and z
        let a = CoordGrid::new(100, 0, 100);
        let b = CoordGrid::new(110, 3, 115);
        assert_eq!(a.distance(b), 15); // max(10, 15)
    }

    // --- Mapsquare boundary tests ---

    #[test]
    fn mapsquare_at_origin() {
        let coord = CoordGrid::new(0, 0, 0);
        assert_eq!(coord.mapsquare_x(), 0);
        assert_eq!(coord.mapsquare_z(), 0);
    }

    #[test]
    fn mapsquare_boundary() {
        // 64 tiles per mapsquare
        let coord = CoordGrid::new(63, 0, 63);
        assert_eq!(coord.mapsquare_x(), 0);
        let coord2 = CoordGrid::new(64, 0, 64);
        assert_eq!(coord2.mapsquare_x(), 1);
    }

    // --- Packed i32 casting (as used in engine) ---

    #[test]
    fn packed_to_i32_and_back() {
        let coord = CoordGrid::new(3200, 1, 3200);
        let as_i32 = coord.packed() as i32;
        let back = CoordGrid(as_i32 as u32);
        assert_eq!(back.x(), 3200);
        assert_eq!(back.y(), 1);
        assert_eq!(back.z(), 3200);
    }

    #[test]
    fn intersects_partial_overlap_x() {
        assert!(CoordGrid::intersects(0, 0, 10, 10, 5, 0, 10, 10));
    }

    #[test]
    fn intersects_partial_overlap_z() {
        assert!(CoordGrid::intersects(0, 0, 10, 10, 0, 5, 10, 10));
    }
}
