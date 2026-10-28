use crate::lifetime::EntityLifeTime;
use rs_grid::{CoordGrid, ZoneCoordGrid};
pub use rs_vm::ObjSerial;

/// Number of ticks after which a private ground object becomes visible to all players.
pub const REVEAL_TICKS: u32 = 100;
/// Sentinel value indicating that a ground object has no specific receiver (visible to all).
pub const NO_RECEIVER: u64 = u64::MAX;

// ---- THE BELOW SECTION IS FOR BUILDING THE ENTIRE PACKED `u128` -- 119 bits wide.

/// Bit offset for the local zone X (`x & 0x7`) within the packed `u128`.
const LOCAL_X_SHIFT: u32 = 0;
/// Bit offset for the local zone Z (`z & 0x7`) within the packed `u128`.
const LOCAL_Z_SHIFT: u32 = 3;
/// Bit offset for the lifetime flag within the packed `u128`.
const LIFETIME_SHIFT: u32 = 6;
/// Bit offset for the obj type id within the packed `u128`.
const ID_SHIFT: u32 = 7;
/// Bit offset for the stack count within the packed `u128`.
const COUNT_SHIFT: u32 = 23;
/// Bit offset for the "has a private receiver" flag within the packed `u128`.
const HAS_RECEIVER_SHIFT: u32 = 54;
/// Bit offset for the `last_clock` field within the packed `u128`.
const LAST_CLOCK_SHIFT: u32 = 55;
/// Bit offset for the zone-unique serial within the packed `u128`.
const SERIAL_SHIFT: u32 = 87;

/// Mask for the 3-bit local-coordinate fields.
const COORD_MASK: u128 = 0x7;
/// Mask for the single-bit lifetime field.
const LIFETIME_MASK: u128 = 0x1;
/// Mask for the 16-bit obj type id.
const ID_MASK: u128 = 0xFFFF;
/// Mask for the 31-bit stack count.
const COUNT_MASK: u128 = 0x7FFFFFFF;
/// Mask for the single-bit "has receiver" flag.
const HAS_RECEIVER_MASK: u128 = 0x1;
/// Mask for the 32-bit `last_clock` field. Its all-ones value is the `u32::MAX`
/// "no timer" sentinel.
const LAST_CLOCK_MASK: u128 = 0xFFFFFFFF;
/// Mask for the 32-bit serial. Bits `119..128` of the `u128` are reserved (unused).
const SERIAL_MASK: u128 = 0xFFFFFFFF;

/// Set in every [`oid`](Obj::oid). A zone keys its queued events by oid for
/// objs and by lid for locs, and a lid never has this bit.
const OID_TAG: u64 = 1 << 63;

/// A ground object (item on the floor) in the game world.
///
/// Every field is bit-packed into a single `u128`, so the struct is exactly one
/// 16-byte. To keep the struct compact, the position is stored as the *local*
/// offset within the owning 8x8 zone (`x & 0x7`, `z & 0x7`); the full world
/// coordinate is reconstructed from the zone's base via
/// [`world_coord`](Self::world_coord).
///
/// The player a private obj is visible to (its base37 "receiver") is *not* stored
/// here: that value is ~63 bits, and most objs are public (no receiver), so storing
/// it inline would bloat every obj for the rare private case. Instead a 1-bit
/// [`has_receiver`](Self::has_receiver) flag marks private objs, and the owning
/// [`Zone`](crate::zone::Zone) keeps the actual receiver in a side map keyed by
/// [`oid`](Self::oid). Public objs never touch that map.
///
/// `last_clock` is a `u32` packed into bits `55..87` (a tick clock; the engine
/// clock is `u32`). Its all-ones value is the `u32::MAX` "no timer" sentinel.
///
/// Packed layout (bit offsets):
/// - `0..3`    local X (`x & 0x7`)
/// - `3..6`    local Z (`z & 0x7`)
/// - `6`       lifetime
/// - `7..23`   obj type id (16 bits)
/// - `23..54`  stack count (31 bits)
/// - `54`      has_receiver flag
/// - `55..87`  last_clock (32 bits)
/// - `87..119` serial (32 bits)
/// - `119..128` reserved
#[derive(Debug, Clone, Copy)]
pub struct Obj(u128);

/// The entire obj state, `last_clock` included, lives in the single `u128`, so the
/// struct is exactly 16 bytes. Guards against accidentally growing it.
const _: () = assert!(size_of::<Obj>() == 16);

impl Obj {
    /// Creates a new ground object at the given coordinate.
    ///
    /// Only the object's position *within its 8x8 zone* is stored; the level and
    /// zone base are recovered from the owning zone. The object starts public (no
    /// receiver flag) and with `last_clock` at its `u32::MAX` sentinel, meaning it
    /// has not yet been scheduled for any state transition. A private receiver, if
    /// any, is recorded by the owning zone (see the type-level docs).
    ///
    /// # Arguments
    /// * `coord` - Grid coordinate where this object is placed (only the intra-zone offset is kept).
    /// * `lifetime` - Whether this object respawns (map-loaded) or despawns (runtime-dropped).
    /// * `id` - The obj type identifier from the config.
    /// * `count` - The stack count of the item.
    ///
    /// # Returns
    /// A new `Obj` with core fields bit-packed and metadata set to sentinel values.
    #[inline(always)]
    pub const fn new(coord: CoordGrid, lifetime: EntityLifeTime, id: u16, count: u32) -> Self {
        let packed = (((coord.x() & COORD_MASK as u16) as u128) << LOCAL_X_SHIFT)
            | (((coord.z() & COORD_MASK as u16) as u128) << LOCAL_Z_SHIFT)
            | ((lifetime as u128) << LIFETIME_SHIFT)
            | ((id as u128) << ID_SHIFT)
            | (((count as u128) & COUNT_MASK) << COUNT_SHIFT)
            | (LAST_CLOCK_MASK << LAST_CLOCK_SHIFT);
        Self(packed)
    }

    /// Returns the obj's local X offset (`0..=7`) within its owning zone.
    #[inline(always)]
    pub const fn local_x(&self) -> u8 {
        ((self.0 >> LOCAL_X_SHIFT) & COORD_MASK) as u8
    }

    /// Returns the obj's local Z offset (`0..=7`) within its owning zone.
    #[inline(always)]
    pub const fn local_z(&self) -> u8 {
        ((self.0 >> LOCAL_Z_SHIFT) & COORD_MASK) as u8
    }

    /// Returns `true` if this obj occupies world tile (`x`, `z`) within its
    /// owning zone -- i.e. their zone-local offsets match. Meant for searches
    /// scoped to the obj's own zone.
    #[inline(always)]
    pub const fn is_at(&self, x: u16, z: u16) -> bool {
        CoordGrid::local_eq(self.local_x(), self.local_z(), x, z)
    }

    /// Reconstructs the obj's full world coordinate from its owning zone's base.
    ///
    /// The obj only stores its intra-zone offset, so the zone's base tile (a
    /// multiple of 8) and level must be supplied -- typically `zone.coord`.
    #[inline(always)]
    pub const fn world_coord(&self, zone: ZoneCoordGrid) -> CoordGrid {
        CoordGrid::new(
            zone.x() | self.local_x() as u16,
            zone.y(),
            zone.z() | self.local_z() as u16,
        )
    }

    /// Returns the lifetime type of this object (respawn or despawn).
    #[inline(always)]
    pub const fn lifetime(&self) -> EntityLifeTime {
        if (self.0 >> LIFETIME_SHIFT) & LIFETIME_MASK == 0 {
            EntityLifeTime::Respawn
        } else {
            EntityLifeTime::Despawn
        }
    }

    /// Returns the obj type identifier from the config.
    #[inline(always)]
    pub const fn id(&self) -> u16 {
        ((self.0 >> ID_SHIFT) & ID_MASK) as u16
    }

    /// Returns the stack count of this object.
    #[inline(always)]
    pub const fn count(&self) -> u32 {
        ((self.0 >> COUNT_SHIFT) & COUNT_MASK) as u32
    }

    /// Sets the stack count of this object.
    ///
    /// Used when stackable drops accumulate onto an existing floor stack.
    #[inline(always)]
    pub const fn set_count(&mut self, count: u32) {
        self.0 = (self.0 & !(COUNT_MASK << COUNT_SHIFT))
            | (((count as u128) & COUNT_MASK) << COUNT_SHIFT);
    }

    /// Returns this obj's [`ObjSerial`], given by the zone when it was inserted.
    #[inline(always)]
    pub const fn serial(&self) -> ObjSerial {
        ObjSerial::from_packed(((self.0 >> SERIAL_SHIFT) & SERIAL_MASK) as u32)
    }

    /// Sets this obj's serial. See [`serial`](Self::serial).
    #[inline(always)]
    pub const fn set_serial(&mut self, serial: ObjSerial) {
        self.0 = (self.0 & !(SERIAL_MASK << SERIAL_SHIFT))
            | (((serial.packed() as u128) & SERIAL_MASK) << SERIAL_SHIFT);
    }

    /// Returns whether this obj is on the floor: a dropped obj until it is
    /// removed, a respawning one unless it was taken and hasn't come back yet.
    ///
    /// The clock doesn't come into it. A drop due to despawn this tick stays
    /// until the zone phase removes it, after the players have had their turn.
    #[inline(always)]
    pub const fn is_active(&self) -> bool {
        matches!(self.lifetime(), EntityLifeTime::Despawn) || self.last_clock() == u32::MAX
    }

    /// Returns whether this obj can be seen, counted or taken at all: on the
    /// floor with something in the stack. Who may see a private one is the
    /// zone's to check.
    #[inline(always)]
    pub const fn is_valid(&self) -> bool {
        self.is_active() && self.count() > 0
    }

    /// Returns whether this obj is privately owned (visible to a single receiver).
    ///
    /// When `true`, the owning zone holds the receiver's base37 id in its side map
    /// keyed by [`oid`](Self::oid). When `false`, the obj is public (no map entry).
    #[inline(always)]
    pub const fn has_receiver(&self) -> bool {
        (self.0 >> HAS_RECEIVER_SHIFT) & HAS_RECEIVER_MASK == 1
    }

    /// Sets the "has a private receiver" flag.
    ///
    /// The zone sets this when it records a receiver in its side map, and clears it
    /// on reveal. See [`has_receiver`](Self::has_receiver).
    #[inline(always)]
    pub const fn set_has_receiver(&mut self, has_receiver: bool) {
        self.0 = (self.0 & !(HAS_RECEIVER_MASK << HAS_RECEIVER_SHIFT))
            | ((has_receiver as u128) << HAS_RECEIVER_SHIFT);
    }

    /// Returns the obj's `last_clock` (tick of its next scheduled state change),
    /// or `u32::MAX` when no timer is pending.
    ///
    /// Stored in bits `63..95` of `packed`; the all-ones value is the `u32::MAX`
    /// "no timer" sentinel, which falls out naturally since the field is exactly 32
    /// bits wide.
    #[inline(always)]
    pub const fn last_clock(&self) -> u32 {
        ((self.0 >> LAST_CLOCK_SHIFT) & LAST_CLOCK_MASK) as u32
    }

    /// Sets the obj's `last_clock`. Pass `u32::MAX` to clear the timer.
    #[inline(always)]
    pub const fn set_last_clock(&mut self, clock: u32) {
        self.0 = (self.0 & !(LAST_CLOCK_MASK << LAST_CLOCK_SHIFT))
            | ((clock as u128) << LAST_CLOCK_SHIFT);
    }

    /// Returns whether this object should be visible at the given engine tick.
    ///
    /// Objects with no pending state change (`last_clock == u32::MAX`) are always
    /// visible. Despawn-type objects are visible while the clock has not yet reached
    /// their `last_clock`. Respawn-type objects become visible once the clock reaches
    /// or exceeds their `last_clock`.
    ///
    /// # Arguments
    /// * `clock` - The current engine tick.
    #[inline(always)]
    pub const fn visible(&self, clock: u32) -> bool {
        if self.last_clock() == u32::MAX {
            return true;
        }
        match self.lifetime() {
            EntityLifeTime::Despawn => clock < self.last_clock(),
            EntityLifeTime::Respawn => clock >= self.last_clock(),
        }
    }

    /// Returns a unique identifier for this object within its zone.
    ///
    /// It is the obj's [`serial`](Self::serial) with a tag bit set, so it never
    /// equals a loc's [`lid`](crate::Loc::lid). A serial isn't handed out twice,
    /// so neither is an oid, even to another obj dropped where this one lay.
    /// Used as the entity key for zone event dedup/cancellation and for the
    /// zone's receiver side map.
    #[inline(always)]
    pub const fn oid(&self) -> u64 {
        OID_TAG | self.serial().packed() as u64
    }

    /// Returns the local zone-relative coordinate packed into a single byte.
    ///
    /// The upper 4 bits hold `x & 0x7` and the lower 4 bits hold `z & 0x7`, suitable
    /// for encoding in network packets.
    #[inline(always)]
    pub const fn packed_zone_coord(&self) -> u8 {
        CoordGrid::packed_zone_coord(self.local_x() as u16, self.local_z() as u16)
    }
}
