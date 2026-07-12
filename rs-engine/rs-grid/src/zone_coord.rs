/// A packed 24-bit zone-aligned coordinate encoding zone X, zone Z, and level (Y)
/// into a single `u32`.
///
/// Zones are 8x8 tile regions, so tile coordinates are right-shifted by 3 before
/// packing. The bit layout is:
///
/// | Bits  | Field           | Width  | Range       |
/// |-------|-----------------|--------|-------------|
/// | 0-10  | zone X (`x>>3`) | 11 bit | 0 - 2047    |
/// | 11-21 | zone Z (`z>>3`) | 11 bit | 0 - 2047    |
/// | 22-23 | level Y         |  2 bit | 0 - 3       |
///
/// Packed formula: `((x>>3) & 0x7FF) | (((z>>3) & 0x7FF) << 11) | ((y & 0x3) << 22)`
///
/// Because tiles are truncated to zone boundaries (multiples of 8), coordinates
/// within the same zone produce the same `ZoneCoordGrid`.
///
/// # Call Stack
///
/// **Called by:** `Engine::track_zone`, `ZoneMap::zone`, `ZoneMap::zone_mut`,
/// `BuildArea::rebuild`, `Zone` tests, `active_player` zone streaming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ZoneCoordGrid(u32);

impl ZoneCoordGrid {
    /// Packs tile coordinates into a zone-aligned coordinate.
    ///
    /// The X and Z values are right-shifted by 3 to convert from tile space to
    /// zone space, discarding the intra-zone offset. This means any tile within
    /// the same 8x8 zone produces an identical `ZoneCoordGrid`.
    ///
    /// # Arguments
    ///
    /// * `x` - Tile X coordinate (0 - 16383). Truncated to zone boundary (`x >> 3`).
    /// * `y` - Level / height plane (0 - 3). Values above 3 are masked.
    /// * `z` - Tile Z coordinate (0 - 16383). Truncated to zone boundary (`z >> 3`).
    ///
    /// # Returns
    ///
    /// A new `ZoneCoordGrid` with the packed representation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::track_zone`, `ZoneMap::zone`, `ZoneMap::zone_mut`,
    /// `BuildArea::rebuild`, `active_player` zone streaming.
    ///
    /// **Calls:** nothing (leaf constructor).
    #[inline(always)]
    pub const fn new(x: u16, y: u8, z: u16) -> Self {
        ZoneCoordGrid(
            (((x >> 3) & 0x7FF) as u32)
                | ((((z >> 3) & 0x7FF) as u32) << 11)
                | (((y & 0x3) as u32) << 22),
        )
    }

    /// Wraps a raw pre-packed `u32` value as a `ZoneCoordGrid`.
    ///
    /// No validation or masking is performed -- the caller is responsible for
    /// ensuring the value follows the expected bit layout. This is typically
    /// used when reading zone keys that were previously packed, such as the
    /// multiway/free2play zone-key sets loaded from the content CSVs.
    ///
    /// # Arguments
    ///
    /// * `packed` - A `u32` already in the `ZoneCoordGrid` bit layout.
    ///
    /// # Returns
    ///
    /// A `ZoneCoordGrid` wrapping the given value directly.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine::new` zone-flag seeding.
    ///
    /// **Calls:** nothing.
    #[inline(always)]
    pub const fn from(packed: u32) -> Self {
        ZoneCoordGrid(packed)
    }

    /// Returns the raw packed `u32` value.
    ///
    /// The value encodes zone X in bits 0-10, zone Z in bits 11-21, and level Y
    /// in bits 22-23. This representation is suitable for use as a hash-map key
    /// or for serialization.
    ///
    /// # Returns
    ///
    /// The internal packed `u32`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** engine zone-tracking collections (`FxHashSet<ZoneCoordGrid>`).
    ///
    /// **Calls:** nothing.
    #[inline(always)]
    pub const fn packed(&self) -> u32 {
        self.0
    }

    /// Unpacks all three components into a `(x, y, z)` tuple.
    ///
    /// Equivalent to calling [`x()`](Self::x), [`y()`](Self::y), and
    /// [`z()`](Self::z) individually, but returned as a single tuple for
    /// convenience when all three are needed at once.
    ///
    /// # Returns
    ///
    /// `(x, y, z)` where X and Z are zone-aligned tile coordinates (multiples
    /// of 8) and Y is the level (0 - 3).
    ///
    /// # Call Stack
    ///
    /// **Called by:** callers needing all three components in a destructure.
    ///
    /// **Calls:** [`Self::x`], [`Self::y`], [`Self::z`].
    #[inline(always)]
    pub const fn index(&self) -> (u16, u8, u16) {
        (self.x(), self.y(), self.z())
    }

    /// Extracts the zone-aligned X tile coordinate.
    ///
    /// The stored 11-bit zone index is left-shifted by 3 to restore it to tile
    /// space, producing a value that is always a multiple of 8.
    ///
    /// # Returns
    ///
    /// Zone-aligned X tile coordinate (0, 8, 16, ..., up to `2047 * 8 = 16376`).
    ///
    /// # Call Stack
    ///
    /// **Called by:** `active_player` zone streaming, `rsmod` collision
    /// allocation checks, [`Self::index`].
    ///
    /// **Calls:** nothing.
    #[inline(always)]
    pub const fn x(&self) -> u16 {
        ((self.0 & 0x7FF) << 3) as u16
    }

    /// Extracts the level (Y / height plane).
    ///
    /// The level is stored in bits 22-23 as a 2-bit value.
    ///
    /// # Returns
    ///
    /// Level value in the range 0 - 3.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `active_player` zone streaming, `rsmod` collision
    /// allocation checks, [`Self::index`].
    ///
    /// **Calls:** nothing.
    #[inline(always)]
    pub const fn y(&self) -> u8 {
        (self.0 >> 22) as u8
    }

    /// Extracts the zone-aligned Z tile coordinate.
    ///
    /// The stored 11-bit zone index is left-shifted by 3 to restore it to tile
    /// space, producing a value that is always a multiple of 8.
    ///
    /// # Returns
    ///
    /// Zone-aligned Z tile coordinate (0, 8, 16, ..., up to `2047 * 8 = 16376`).
    ///
    /// # Call Stack
    ///
    /// **Called by:** `active_player` zone streaming, `rsmod` collision
    /// allocation checks, [`Self::index`].
    ///
    /// **Calls:** nothing.
    #[inline(always)]
    pub const fn z(&self) -> u16 {
        (((self.0 >> 11) & 0x7FF) << 3) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_accessors() {
        let zone = ZoneCoordGrid::new(3200, 1, 3200);
        assert_eq!(zone.x(), (3200 >> 3) << 3);
        assert_eq!(zone.y(), 1);
        assert_eq!(zone.z(), (3200 >> 3) << 3);
    }

    #[test]
    fn from_packed_round_trips() {
        let zone = ZoneCoordGrid::new(3200, 2, 3200);
        let wrapped = ZoneCoordGrid::from(zone.packed());
        assert_eq!(wrapped, zone);
        assert_eq!(wrapped.index(), (3200, 2, 3200));
    }

    #[test]
    fn index_tuple() {
        let zone = ZoneCoordGrid::new(3200, 2, 3200);
        let (x, y, z) = zone.index();
        assert_eq!(x, zone.x());
        assert_eq!(y, zone.y());
        assert_eq!(z, zone.z());
    }

    #[test]
    fn zone_coord_truncates_to_zone_boundary() {
        let zone = ZoneCoordGrid::new(3205, 0, 3207);
        assert_eq!(zone.x(), 3200);
        assert_eq!(zone.z(), 3200);
    }

    #[test]
    fn y_wraps_at_4() {
        let zone = ZoneCoordGrid::new(0, 3, 0);
        assert_eq!(zone.y(), 3);
        let zone2 = ZoneCoordGrid::new(0, 4, 0);
        assert_eq!(zone2.y(), 0);
    }

    #[test]
    fn zero_coords() {
        let zone = ZoneCoordGrid::new(0, 0, 0);
        assert_eq!(zone.x(), 0);
        assert_eq!(zone.y(), 0);
        assert_eq!(zone.z(), 0);
    }

    #[test]
    fn max_zone_values() {
        let max_11 = 0x7FF;
        let zone = ZoneCoordGrid::new(max_11 << 3, 3, max_11 << 3);
        assert_eq!(zone.x(), max_11 << 3);
        assert_eq!(zone.z(), max_11 << 3);
    }

    #[test]
    fn equality_and_hash() {
        let a = ZoneCoordGrid::new(3200, 1, 3200);
        let b = ZoneCoordGrid::new(3200, 1, 3200);
        let c = ZoneCoordGrid::new(3200, 2, 3200);
        assert_eq!(a, b);
        assert_ne!(a, c);

        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(a);
        assert!(set.contains(&b));
        assert!(!set.contains(&c));
    }

    #[test]
    fn copy_clone() {
        let a = ZoneCoordGrid::new(3200, 1, 3200);
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn various_zone_coordinates() {
        for x in [0u16, 8, 64, 3200, 16376] {
            for y in [0u8, 1, 2, 3] {
                for z in [0u16, 8, 64, 3200, 16376] {
                    let zone = ZoneCoordGrid::new(x, y, z);
                    let expected_x = (x >> 3) << 3;
                    let expected_z = (z >> 3) << 3;
                    assert_eq!(zone.x(), expected_x, "x mismatch for ({x},{y},{z})");
                    assert_eq!(zone.y(), y, "y mismatch for ({x},{y},{z})");
                    assert_eq!(zone.z(), expected_z, "z mismatch for ({x},{y},{z})");
                }
            }
        }
    }

    #[test]
    fn zone_coord_from_coord_grid() {
        use crate::CoordGrid;
        let coord = CoordGrid::new(3200, 1, 3200);
        let zone = ZoneCoordGrid::new(coord.x(), coord.y(), coord.z());
        assert_eq!(zone.y(), 1);
        assert_eq!(zone.x(), (3200 >> 3) << 3);
        assert_eq!(zone.z(), (3200 >> 3) << 3);
    }

    #[test]
    fn zone_coord_alignment() {
        // Coords within the same zone should produce the same ZoneCoordGrid
        let a = ZoneCoordGrid::new(3200, 0, 3200);
        let b = ZoneCoordGrid::new(3207, 0, 3207);
        assert_eq!(a, b);
    }

    #[test]
    fn zone_coord_different_zones() {
        let a = ZoneCoordGrid::new(3200, 0, 3200);
        let b = ZoneCoordGrid::new(3208, 0, 3200);
        assert_ne!(a, b);
    }
}
