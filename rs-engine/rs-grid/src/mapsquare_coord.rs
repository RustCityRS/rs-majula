/// A packed 14-bit coordinate for positions local to a single 64x64 mapsquare.
///
/// Encodes a mapsquare-local X, Z, and level (Y) into a single `u16`. Each
/// mapsquare is 64 tiles wide and 64 tiles tall, so X and Z each fit in 6 bits,
/// and the level fits in 2 bits.
///
/// | Bits  | Field   | Width  | Range  |
/// |-------|---------|--------|--------|
/// | 0-5   | Z       | 6 bit  | 0 - 63 |
/// | 6-11  | X       | 6 bit  | 0 - 63 |
/// | 12-13 | level Y | 2 bit  | 0 - 3  |
///
/// Packed formula: `(z & 0x3F) | ((x & 0x3F) << 6) | ((y & 0x3) << 12)`
///
/// The packed value can be used directly as an array index (values 0 - 16383)
/// for per-tile lookup tables within a mapsquare.
///
/// # Call Stack
///
/// **Called by:** `GameMap` land / loc / obj decoding routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MapsquareCoordGrid(u16);

impl MapsquareCoordGrid {
    /// Packs mapsquare-local components into a coordinate.
    ///
    /// Each component is masked to its valid range before packing, so values
    /// that exceed the field width wrap silently (e.g. `x = 64` becomes `0`).
    ///
    /// # Arguments
    ///
    /// * `x` - Local X offset within the mapsquare (0 - 63). Masked to 6 bits.
    /// * `y` - Level / height plane (0 - 3). Masked to 2 bits.
    /// * `z` - Local Z offset within the mapsquare (0 - 63). Masked to 6 bits.
    ///
    /// # Returns
    ///
    /// A new `MapsquareCoordGrid` with the packed representation.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `GameMap::load_lands`, `GameMap::load_locs`,
    /// `GameMap::load_objs` (constructing per-tile coordinates during map
    /// decoding); also used to create bridge-check coordinates at level 1.
    ///
    /// **Calls:** nothing (leaf constructor).
    #[inline(always)]
    pub const fn new(x: u8, y: u8, z: u8) -> Self {
        MapsquareCoordGrid(
            ((z & 0x3F) as u16) | (((x & 0x3F) as u16) << 6) | (((y & 0x3) as u16) << 12),
        )
    }

    /// Wraps a raw pre-packed `u16` value into a `MapsquareCoordGrid`.
    ///
    /// No validation or masking is performed; the caller is responsible for
    /// ensuring the value was produced by the same bit layout. This is typically
    /// used when reading a coordinate offset from a sequential decoder where the
    /// packed integer has been accumulated arithmetically.
    ///
    /// # Arguments
    ///
    /// * `packed` - A pre-packed 14-bit coordinate value.
    ///
    /// # Returns
    ///
    /// A `MapsquareCoordGrid` wrapping the provided value.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `GameMap::load_locs`, `GameMap::load_objs` (converting an
    /// accumulated coordinate offset back into a typed coordinate).
    ///
    /// **Calls:** nothing (leaf constructor).
    #[inline(always)]
    pub const fn from(packed: u16) -> Self {
        MapsquareCoordGrid(packed)
    }

    /// Returns the raw packed `u16` value.
    ///
    /// The value can be used directly as an array index into per-tile lookup
    /// tables (e.g. the `lands` collision-flag array in `GameMap`). Valid
    /// packed values range from 0 to 16383.
    ///
    /// # Returns
    ///
    /// The internal packed `u16`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `GameMap::load_lands`, `GameMap::load_locs` (indexing
    /// into the per-mapsquare `lands` array for collision / bridge flags).
    ///
    /// **Calls:** nothing.
    #[inline(always)]
    pub const fn packed(&self) -> u16 {
        self.0
    }

    /// Extracts the local X offset within the mapsquare.
    ///
    /// Reads bits 6-11 of the packed value.
    ///
    /// # Returns
    ///
    /// X offset in the range 0 - 63.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `GameMap::load_locs` (reconstructing bridge-check
    /// coordinates), callers converting to absolute tile coordinates.
    ///
    /// **Calls:** nothing.
    #[inline(always)]
    pub const fn x(self) -> u8 {
        ((self.0 >> 6) & 0x3F) as u8
    }

    /// Extracts the level (Y / height plane).
    ///
    /// Reads bits 12-13 of the packed value.
    ///
    /// # Returns
    ///
    /// Level value in the range 0 - 3.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `GameMap::load_locs` (checking bridge status at the
    /// decoded coordinate's level).
    ///
    /// **Calls:** nothing.
    #[inline(always)]
    pub const fn y(self) -> u8 {
        ((self.0 >> 12) & 0x3) as u8
    }

    /// Extracts the local Z offset within the mapsquare.
    ///
    /// Reads bits 0-5 of the packed value.
    ///
    /// # Returns
    ///
    /// Z offset in the range 0 - 63.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `GameMap::load_locs` (reconstructing bridge-check
    /// coordinates), callers converting to absolute tile coordinates.
    ///
    /// **Calls:** nothing.
    #[inline(always)]
    pub const fn z(self) -> u8 {
        (self.0 & 0x3F) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_accessors() {
        let ms = MapsquareCoordGrid::new(50, 1, 50);
        assert_eq!(ms.x(), 50);
        assert_eq!(ms.y(), 1);
        assert_eq!(ms.z(), 50);
    }

    #[test]
    fn default_is_zero() {
        let ms = MapsquareCoordGrid::default();
        assert_eq!(ms.x(), 0);
        assert_eq!(ms.y(), 0);
        assert_eq!(ms.z(), 0);
    }

    #[test]
    fn max_6_bit_values() {
        let max_6 = 0x3F; // 63
        let ms = MapsquareCoordGrid::new(max_6, 3, max_6);
        assert_eq!(ms.x(), max_6);
        assert_eq!(ms.y(), 3);
        assert_eq!(ms.z(), max_6);
    }

    #[test]
    fn overflow_masked() {
        let ms = MapsquareCoordGrid::new(64, 4, 64);
        assert_eq!(ms.x(), 0);
        assert_eq!(ms.y(), 0);
        assert_eq!(ms.z(), 0);
    }

    #[test]
    fn y_wraps_at_4() {
        let ms = MapsquareCoordGrid::new(0, 3, 0);
        assert_eq!(ms.y(), 3);
        let ms2 = MapsquareCoordGrid::new(0, 5, 0);
        assert_eq!(ms2.y(), 1);
    }

    #[test]
    fn equality_and_hash() {
        let a = MapsquareCoordGrid::new(50, 1, 50);
        let b = MapsquareCoordGrid::new(50, 1, 50);
        let c = MapsquareCoordGrid::new(50, 2, 50);
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
    fn copy_clone() {
        let a = MapsquareCoordGrid::new(10, 2, 30);
        let b = a;
        let c = a.clone();
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn packed_representation() {
        let ms = MapsquareCoordGrid::new(10, 2, 20);
        let packed = ms.0;
        let reconstructed = MapsquareCoordGrid(packed);
        assert_eq!(reconstructed.x(), 10);
        assert_eq!(reconstructed.y(), 2);
        assert_eq!(reconstructed.z(), 20);
    }

    #[test]
    fn various_coordinates() {
        for x in [0u8, 1, 31, 50, 63] {
            for y in [0u8, 1, 2, 3] {
                for z in [0u8, 1, 31, 50, 63] {
                    let ms = MapsquareCoordGrid::new(x, y, z);
                    assert_eq!(ms.x(), x, "x mismatch for ({x},{y},{z})");
                    assert_eq!(ms.y(), y, "y mismatch for ({x},{y},{z})");
                    assert_eq!(ms.z(), z, "z mismatch for ({x},{y},{z})");
                }
            }
        }
    }

    #[test]
    fn mapsquare_from_coord_grid() {
        use crate::CoordGrid;
        let coord = CoordGrid::new(3200, 1, 3200);
        let ms = MapsquareCoordGrid::new(
            coord.mapsquare_x() as u8,
            coord.y(),
            coord.mapsquare_z() as u8,
        );
        assert_eq!(ms.x(), 50);
        assert_eq!(ms.y(), 1);
        assert_eq!(ms.z(), 50);
    }

    #[test]
    fn mapsquare_all_zeros() {
        let ms = MapsquareCoordGrid::new(0, 0, 0);
        assert_eq!(ms.0, 0);
    }

    #[test]
    fn mapsquare_adjacent() {
        let a = MapsquareCoordGrid::new(50, 0, 50);
        let b = MapsquareCoordGrid::new(51, 0, 50);
        assert_ne!(a, b);
    }
}
