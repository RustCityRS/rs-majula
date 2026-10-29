use crate::lifetime::EntityLifeTime;
use rs_grid::{CoordGrid, ZoneCoordGrid};
use rs_pack::types::{LocAngle, LocLayer, LocShape};
pub use rs_vm::LocSerial;

// ---- THE BELOW SECTION IS FOR BUILDING THE ENTIRE PACKED `u128` -- all 128 bits.

/// Bit offset for the local zone X (`x & 0x7`) within the packed `u128`.
const LOCAL_X_SHIFT: u32 = 0;
/// Bit offset for the local zone Z (`z & 0x7`) within the packed `u128`.
const LOCAL_Z_SHIFT: u32 = 3;
/// Bit offset for the lifetime flag within the packed `u128`.
const LIFETIME_SHIFT: u32 = 6;
/// Bit offset for the base (original) info within the packed `u128`.
const BASE_INFO_SHIFT: u32 = 7;
/// Bit offset for the current (possibly modified) info within the packed `u128`.
const CURRENT_INFO_SHIFT: u32 = 44;
/// Bit offset for the `last_clock` field within the packed `u128`.
const LAST_CLOCK_SHIFT: u32 = 81;
/// Bit offset for the removed flag within the packed `u128`.
const REMOVED_SHIFT: u32 = 113;
/// Bit offset for the [`LocSerial`] within the packed `u128`.
const SERIAL_SHIFT: u32 = 114;

/// Mask for the 3-bit local-coordinate fields.
const COORD_MASK: u128 = 0x7;
/// Mask for the single-bit lifetime field.
const LIFETIME_MASK: u128 = 0x1;
/// Mask for the 37-bit info field (id, shape, angle, blockwalk, blockrange, width, length).
const INFO_MASK: u128 = 0x1FFFFFFFFF;
/// Mask for the 32-bit `last_clock` field. Its all-ones value is the `u32::MAX`
/// "no timer" sentinel.
const LAST_CLOCK_MASK: u128 = 0xFFFFFFFF;
/// Mask for the single-bit removed flag.
const REMOVED_MASK: u128 = 0x1;
/// Mask for the [`LocSerial`] field.
const SERIAL_MASK: u128 = (1 << LocSerial::BITS) - 1;

// ---- THE BELOW SECTION IS FOR BUILDING THE INDIVIDUAL INFO BITS `u64` -- 37 bits wide.

/// Bit offset for the loc type id within the info word.
const ID_SHIFT: u32 = 0;
/// Bit offset for the shape within the info word.
const SHAPE_SHIFT: u32 = 16;
/// Bit offset for the angle within the info word.
const ANGLE_SHIFT: u32 = 21;
/// Bit offset for the blockwalk flag within the info word.
const BLOCKWALK_SHIFT: u32 = 23;
/// Bit offset for the blockrange flag within the info word.
const BLOCKRANGE_SHIFT: u32 = 24;
/// Bit offset for the width field within the info word.
const WIDTH_SHIFT: u32 = 25;
/// Bit offset for the length field within the info word.
const LENGTH_SHIFT: u32 = 31;

/// Mask for the 16-bit loc type id.
const ID_MASK: u64 = 0xFFFF;
/// Mask for the 5-bit shape.
const SHAPE_MASK: u64 = 0x1F;
/// Mask for the 2-bit angle.
const ANGLE_MASK: u64 = 0x3;
/// Mask for the single-bit blockwalk flag.
const BLOCKWALK_MASK: u64 = 0x1;
/// Mask for the single-bit blockrange flag.
const BLOCKRANGE_MASK: u64 = 0x1;
/// Mask for the 6-bit width field.
const WIDTH_MASK: u64 = 0x3F;
/// Mask for the 6-bit length field.
const LENGTH_MASK: u64 = 0x3F;

/// A placed location (scenery/object) in the game world.
///
/// Every field is bit-packed into a single `u128` -- including `last_clock`, so the
/// struct is exactly one 16-byte. The position is stored as the *local* offset
/// within the owning 8x8 zone (`x & 0x7`, `z & 0x7`); the full world coordinate is
/// reconstructed from the zone's base via [`world_coord`](Self::world_coord). The
/// struct tracks both base and current info to support runtime changes (e.g. opening
/// a door changes its `id`) while preserving the original state for revert on
/// respawn-type locs.
///
/// Each info holds `id`, `shape`, `angle`, the `blockwalk`/`blockrange` flags,
/// and `width`/`length`. These are *per-info* (base and current each carry their
/// own) because a runtime change can swap the loc for a different type whose
/// dimensions, shape, or flags differ -- e.g. lighting a fire (centrepiece) on a
/// tile, then closing a diagonal door (wall) over it. The client renders by
/// `(id, shape)`, so the changed shape must reach it or the new loc renders with no
/// matching model (i.e. invisibly); collision uses the current width/length.
/// [`revert`](Self::revert) restores the entire base info (id, shape, angle, flags,
/// dimensions) in one shot.
///
/// The collision layer is *not* stored: it is a pure function of the *base* shape
/// (see [`LocShape::layer`]) and is derived on demand by [`layer`](Self::layer).
/// The base shape is used (not the current one) so the layer is stable across
/// runtime changes -- matching the reference engine, where `loc.layer` reads the
/// base info. A change only ever swaps a loc for another on the *same* layer, so the
/// current shape always shares that layer.
///
/// `last_clock` is a `u32` packed into bits `81..113` (a tick clock; the engine
/// clock is `u32`). Its all-ones value is the `u32::MAX` "no timer" sentinel,
/// returned by [`last_clock`](Self::last_clock) / set by
/// [`set_last_clock`](Self::set_last_clock).
///
/// Bit `113` marks a removed map loc, which stays in its zone until it
/// respawns. It keeps the type it had when removed, as in the reference
/// engine, so the flag can't be read off the info or the clock. Bits `114..128`
/// hold the [`LocSerial`] the zone gave it.
///
/// Layout (bit offsets):
/// - `0..3`    local X (`x & 0x7`)
/// - `3..6`    local Z (`z & 0x7`)
/// - `6`       lifetime
/// - `7..44`   base info    (`id16 | shape5 | angle2 | blockwalk1 | blockrange1 | width6 | length6`)
/// - `44..81`  current info (`id16 | shape5 | angle2 | blockwalk1 | blockrange1 | width6 | length6`)
/// - `81..113` last_clock (32 bits)
/// - `113`     removed
/// - `114..128` serial (14 bits)
#[derive(Debug, Clone, Copy)]
pub struct Loc(u128);

/// The entire loc state, `last_clock` included, lives in the single `u128`, so the
/// struct is exactly 16 bytes. Guards against accidentally growing it.
const _: () = assert!(size_of::<Loc>() == 16);

impl Loc {
    /// Creates a new `Loc` with the given position, lifetime, and type info.
    ///
    /// Only the loc's position *within its 8x8 zone* (`coord.x() & 0x7`,
    /// `coord.z() & 0x7`) is stored; the level and zone base are recovered from the
    /// owning zone. Both the base info and current info are initialized to the
    /// same values, `last_clock` is set to its `u32::MAX` sentinel (not yet
    /// modified), and the loc is visible with serial [`LocSerial::FIRST`] until its
    /// zone gives it one.
    ///
    /// # Arguments
    /// * `coord` - Grid coordinate where this loc is placed (only the intra-zone offset is kept).
    /// * `lifetime` - Whether this loc respawns (map-loaded) or despawns (runtime-spawned).
    /// * `id` - The loc type identifier from the config.
    /// * `shape` - The rendering shape of the loc (e.g., wall, centrepiece).
    /// * `angle` - The rotation angle of the loc.
    /// * `blockwalk` - Whether the loc blocks movement (from its loc type).
    /// * `blockrange` - Whether the loc blocks ranged projectiles (from its loc type).
    /// * `width` - Width of the loc in tiles (x-axis); clamped to `0..=63`.
    /// * `length` - Length of the loc in tiles (z-axis); clamped to `0..=63`.
    ///
    /// # Returns
    /// A new `Loc` with all fields bit-packed.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        coord: CoordGrid,
        lifetime: EntityLifeTime,
        id: u16,
        shape: LocShape,
        angle: LocAngle,
        blockwalk: bool,
        blockrange: bool,
        width: u8,
        length: u8,
    ) -> Self {
        let info = Self::pack_info(id, shape, angle, blockwalk, blockrange, width, length) as u128;
        let packed = (((coord.x() & COORD_MASK as u16) as u128) << LOCAL_X_SHIFT)
            | (((coord.z() & COORD_MASK as u16) as u128) << LOCAL_Z_SHIFT)
            | ((lifetime as u128) << LIFETIME_SHIFT)
            | (info << BASE_INFO_SHIFT)
            | (info << CURRENT_INFO_SHIFT)
            | (LAST_CLOCK_MASK << LAST_CLOCK_SHIFT);
        Self(packed)
    }

    /// Returns whether this loc is in the world.
    ///
    /// A despawn-type loc leaves its zone when removed, so it is visible while
    /// stored. A respawn-type (map) loc stays stored while removed and is
    /// hidden until it respawns or a change brings it back.
    ///
    /// # Returns
    /// `true` if the loc should be sent to nearby players.
    #[inline(always)]
    pub const fn visible(&self) -> bool {
        (self.0 >> REMOVED_SHIFT) & REMOVED_MASK == 0
    }

    /// Shows or hides this loc. Hiding leaves its type alone, so a removed map
    /// loc reads as the type it had when removed until it respawns.
    #[inline(always)]
    pub const fn set_visible(&mut self, visible: bool) {
        self.0 =
            (self.0 & !(REMOVED_MASK << REMOVED_SHIFT)) | ((!visible as u128) << REMOVED_SHIFT);
    }

    /// Returns this loc's [`LocSerial`], given by the zone when it was inserted.
    #[inline(always)]
    pub const fn serial(&self) -> LocSerial {
        LocSerial::from_packed(((self.0 >> SERIAL_SHIFT) & SERIAL_MASK) as u16)
    }

    /// Sets this loc's [`LocSerial`]. Only the zone does this, as the loc goes in.
    #[inline(always)]
    pub const fn set_serial(&mut self, serial: LocSerial) {
        self.0 = (self.0 & !(SERIAL_MASK << SERIAL_SHIFT))
            | ((serial.packed() as u128 & SERIAL_MASK) << SERIAL_SHIFT);
    }

    /// Returns the loc's local X offset (`0..=7`) within its owning zone.
    #[inline(always)]
    pub const fn local_x(&self) -> u8 {
        ((self.0 >> LOCAL_X_SHIFT) & COORD_MASK) as u8
    }

    /// Returns the loc's local Z offset (`0..=7`) within its owning zone.
    #[inline(always)]
    pub const fn local_z(&self) -> u8 {
        ((self.0 >> LOCAL_Z_SHIFT) & COORD_MASK) as u8
    }

    /// Returns `true` if this loc occupies world tile (`x`, `z`) within its
    /// owning zone -- i.e. their zone-local offsets match. Meant for searches
    /// scoped to the loc's own zone.
    #[inline(always)]
    pub const fn is_at(&self, x: u16, z: u16) -> bool {
        CoordGrid::local_eq(self.local_x(), self.local_z(), x, z)
    }

    /// Reconstructs the loc's full world coordinate from its owning zone's base.
    ///
    /// The loc only stores its intra-zone offset, so the zone's base tile (a
    /// multiple of 8) and level must be supplied -- typically `zone.coord`.
    #[inline(always)]
    pub const fn world_coord(&self, zone: ZoneCoordGrid) -> CoordGrid {
        CoordGrid::new(
            zone.x() | self.local_x() as u16,
            zone.y(),
            zone.z() | self.local_z() as u16,
        )
    }

    /// Returns the lifetime type of this loc (respawn or despawn).
    #[inline(always)]
    pub const fn lifetime(&self) -> EntityLifeTime {
        if (self.0 >> LIFETIME_SHIFT) & LIFETIME_MASK == 0 {
            EntityLifeTime::Respawn
        } else {
            EntityLifeTime::Despawn
        }
    }

    /// Returns the current loc type identifier.
    ///
    /// This reflects the current info, which may differ from the base info if the
    /// loc has been changed at runtime.
    #[inline(always)]
    pub const fn id(&self) -> u16 {
        (self.current_info() & ID_MASK) as u16
    }

    /// Returns the current rendering shape of this loc.
    ///
    /// Read from the current info, so it reflects a runtime change (e.g. a tile's
    /// loc being swapped for a different-shaped one on the same layer); after a
    /// [`revert`](Self::revert) it matches the base shape again. The client renders
    /// a loc by `(id, shape)`, so this must carry the changed shape -- otherwise the
    /// new loc has no matching model and renders invisibly.
    ///
    /// # Safety
    /// Uses `transmute` internally; the 5-bit value is assumed to be a valid `LocShape` discriminant.
    #[inline(always)]
    pub const fn shape(&self) -> LocShape {
        unsafe { std::mem::transmute(((self.current_info() >> SHAPE_SHIFT) & SHAPE_MASK) as u8) }
    }

    /// Returns the original (base) shape of this loc, ignoring any runtime change.
    ///
    /// Used to derive the collision [`layer`](Self::layer), which must stay stable
    /// across changes (a change only swaps a loc within the same layer).
    ///
    /// # Safety
    /// Uses `transmute` internally; the 5-bit value is assumed to be a valid `LocShape` discriminant.
    #[inline(always)]
    const fn base_shape(&self) -> LocShape {
        unsafe { std::mem::transmute(((self.base_info() >> SHAPE_SHIFT) & SHAPE_MASK) as u8) }
    }

    /// Returns the current rotation angle of this loc.
    ///
    /// Read from the current info, so it reflects a runtime change (e.g. a door
    /// rotating); after a [`revert`](Self::revert) it matches the base again.
    ///
    /// # Safety
    /// Uses `transmute` internally; the 2-bit value is assumed to be a valid `LocAngle` discriminant.
    #[inline(always)]
    pub const fn angle(&self) -> LocAngle {
        unsafe { std::mem::transmute(((self.current_info() >> ANGLE_SHIFT) & ANGLE_MASK) as u8) }
    }

    /// Returns whether this loc currently blocks movement.
    ///
    /// Read from the current info so it reflects any runtime change; after a
    /// [`revert`](Self::revert) it again matches the base loc type.
    #[inline(always)]
    pub const fn blockwalk(&self) -> bool {
        (self.current_info() >> BLOCKWALK_SHIFT) & BLOCKWALK_MASK == 1
    }

    /// Returns whether this loc currently blocks ranged projectiles.
    ///
    /// Read from the current info so it reflects any runtime change; after a
    /// [`revert`](Self::revert) it again matches the base loc type.
    #[inline(always)]
    pub const fn blockrange(&self) -> bool {
        (self.current_info() >> BLOCKRANGE_SHIFT) & BLOCKRANGE_MASK == 1
    }

    /// Returns the width of the loc in tiles along the x-axis (`0..=63`).
    ///
    /// Read from the current info; reflects any runtime change and is restored to
    /// the base value by [`revert`](Self::revert).
    #[inline(always)]
    pub const fn width(&self) -> u8 {
        ((self.current_info() >> WIDTH_SHIFT) & WIDTH_MASK) as u8
    }

    /// Returns the length of the loc in tiles along the z-axis (`0..=63`).
    ///
    /// Read from the current info; reflects any runtime change and is restored to
    /// the base value by [`revert`](Self::revert).
    #[inline(always)]
    pub const fn length(&self) -> u8 {
        ((self.current_info() >> LENGTH_SHIFT) & LENGTH_MASK) as u8
    }

    /// Returns the collision layer of this loc.
    ///
    /// The layer is derived from the *base* shape ([`LocShape::layer`]) rather
    /// than stored. Using the base shape keeps the layer stable across runtime
    /// changes (a change only swaps a loc within the same layer), matching the
    /// reference engine where `loc.layer` reads the base info.
    #[inline(always)]
    pub const fn layer(&self) -> LocLayer {
        self.base_shape().layer()
    }

    /// Returns the loc's `last_clock` (tick of its next scheduled state change),
    /// or `u32::MAX` when no timer is pending.
    ///
    /// Stored in bits `81..113` of `packed`. The all-ones value is the `u32::MAX`
    /// "no timer" sentinel, which falls out naturally since the field is exactly 32
    /// bits wide.
    #[inline(always)]
    pub const fn last_clock(&self) -> u32 {
        ((self.0 >> LAST_CLOCK_SHIFT) & LAST_CLOCK_MASK) as u32
    }

    /// Sets the loc's `last_clock`. Pass `u32::MAX` to clear the timer.
    ///
    /// The value is stored verbatim in bits `81..113` of `packed`; `u32::MAX` is the
    /// "no timer" sentinel.
    #[inline(always)]
    pub const fn set_last_clock(&mut self, clock: u32) {
        self.0 = (self.0 & !(LAST_CLOCK_MASK << LAST_CLOCK_SHIFT))
            | ((clock as u128) << LAST_CLOCK_SHIFT);
    }

    /// Returns `true` if the current info differs from the base info.
    ///
    /// A changed loc needs to be communicated to clients so they see the updated
    /// type, shape, angle, flags or dimensions.
    #[inline(always)]
    pub const fn is_changed(&self) -> bool {
        self.current_info() != self.base_info()
    }

    /// Modifies the current info of this loc to a new type id, shape, angle, flags
    /// and dimensions.
    ///
    /// The base info is left unchanged, allowing the loc to be reverted later.
    /// This is used for runtime loc changes -- e.g. opening/closing doors (which
    /// rotate the loc), or swapping a tile's loc for a different-shaped one on the
    /// same layer (lighting a fire, then closing a diagonal door over it). The shape
    /// *is* updated: the client renders by `(id, shape)`, so a changed shape must be
    /// reflected here or the new loc renders with no matching model. The new type's
    /// `width`/`length` are stored too, since they can differ from the base type's.
    ///
    /// # Arguments
    /// * `id` - New loc type identifier.
    /// * `shape` - New rendering shape.
    /// * `angle` - New rotation angle.
    /// * `blockwalk` - Whether the new loc type blocks movement.
    /// * `blockrange` - Whether the new loc type blocks ranged projectiles.
    /// * `width` - New width in tiles (x-axis); clamped to `0..=63`.
    /// * `length` - New length in tiles (z-axis); clamped to `0..=63`.
    ///
    /// # Side Effects
    /// * Overwrites the current info bits in `self.packed`.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    pub const fn change(
        &mut self,
        id: u16,
        shape: LocShape,
        angle: LocAngle,
        blockwalk: bool,
        blockrange: bool,
        width: u8,
        length: u8,
    ) {
        let info = Self::pack_info(id, shape, angle, blockwalk, blockrange, width, length) as u128;
        self.0 = (self.0 & !(INFO_MASK << CURRENT_INFO_SHIFT)) | (info << CURRENT_INFO_SHIFT);
    }

    /// Reverts the current info back to the base info.
    ///
    /// Used when a respawn-type loc's change timer expires, restoring it to its
    /// original map-loaded state (e.g., a door closing automatically). Restores
    /// id, shape, angle, flags and dimensions in one shot; `last_clock` is left
    /// untouched.
    ///
    /// # Side Effects
    /// * Copies the base info bits into the current info bits in `self.packed`.
    #[inline(always)]
    pub const fn revert(&mut self) {
        let base = (self.0 >> BASE_INFO_SHIFT) & INFO_MASK;
        self.0 = (self.0 & !(INFO_MASK << CURRENT_INFO_SHIFT)) | (base << CURRENT_INFO_SHIFT);
    }

    /// Returns a unique local identifier for this loc within its zone.
    ///
    /// The identifier is composed of the local x and z coordinates (3 bits each)
    /// and the layer (2 bits), packed into a `u64`.
    #[inline(always)]
    pub const fn lid(&self) -> u64 {
        ((self.local_x() as u64) << LOCAL_X_SHIFT)
            | ((self.local_z() as u64) << LOCAL_Z_SHIFT)
            | ((self.layer() as u64 & 0x3) << 6)
    }

    /// Returns the local zone-relative coordinate packed into a single byte.
    ///
    /// The upper 4 bits hold `x & 0x7` and the lower 4 bits hold `z & 0x7`, suitable
    /// for encoding in network packets.
    #[inline(always)]
    pub const fn packed_zone_coord(&self) -> u8 {
        CoordGrid::packed_zone_coord(self.local_x() as u16, self.local_z() as u16)
    }

    /// Returns the shape and angle packed into a single byte for network encoding.
    ///
    /// The upper 6 bits hold the shape (shifted left by 2) and the lower 2 bits hold
    /// the angle.
    #[inline(always)]
    pub const fn packed_shape_angle(&self) -> u8 {
        ((self.shape() as u8) << 2) | (self.angle() as u8)
    }

    /// Packs a loc type id, shape, angle, collision flags and dimensions into a
    /// 37-bit info value.
    ///
    /// Layout: bits 0-15 = id, bits 16-20 = shape, bits 21-22 = angle,
    /// bit 23 = blockwalk, bit 24 = blockrange, bits 25-30 = width,
    /// bits 31-36 = length. `width`/`length` are clamped to `0..=63`.
    #[inline(always)]
    #[allow(clippy::too_many_arguments)]
    const fn pack_info(
        id: u16,
        shape: LocShape,
        angle: LocAngle,
        blockwalk: bool,
        blockrange: bool,
        width: u8,
        length: u8,
    ) -> u64 {
        let width = if width as u64 > WIDTH_MASK {
            WIDTH_MASK as u8
        } else {
            width
        };
        let length = if length as u64 > LENGTH_MASK {
            LENGTH_MASK as u8
        } else {
            length
        };
        ((id as u64 & ID_MASK) << ID_SHIFT)
            | ((shape as u64 & SHAPE_MASK) << SHAPE_SHIFT)
            | ((angle as u64 & ANGLE_MASK) << ANGLE_SHIFT)
            | ((blockwalk as u64 & BLOCKWALK_MASK) << BLOCKWALK_SHIFT)
            | ((blockrange as u64 & BLOCKRANGE_MASK) << BLOCKRANGE_SHIFT)
            | ((width as u64 & WIDTH_MASK) << WIDTH_SHIFT)
            | ((length as u64 & LENGTH_MASK) << LENGTH_SHIFT)
    }

    /// Extracts the base (original) info from the packed representation.
    #[inline(always)]
    const fn base_info(&self) -> u64 {
        ((self.0 >> BASE_INFO_SHIFT) & INFO_MASK) as u64
    }

    /// Extracts the current (possibly modified) info from the packed representation.
    #[inline(always)]
    const fn current_info(&self) -> u64 {
        ((self.0 >> CURRENT_INFO_SHIFT) & INFO_MASK) as u64
    }
}
