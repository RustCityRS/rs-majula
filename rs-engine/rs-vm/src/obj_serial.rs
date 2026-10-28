/// Names one ground obj within its zone.
///
/// The zone hands serials out in turn as objs are inserted, and an obj keeps
/// its serial while it is taken and respawns, restacks or is revealed. A
/// reference holding one (a script's active obj, an interaction target) finds
/// that obj only, never another dropped on the same tile later: serials come
/// back round only after 4 billion objs in one zone.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ObjSerial(u32);

impl ObjSerial {
    /// The first serial a zone hands out.
    pub const FIRST: Self = Self(0);

    /// A serial no obj is given: a reference to it, like one to a drop that
    /// merged into a stack already there, never finds anything.
    pub const NONE: Self = Self(u32::MAX);

    /// Rebuilds a serial from the bits an obj packs it into.
    #[inline(always)]
    pub const fn from_packed(packed: u32) -> Self {
        Self(packed)
    }

    /// Returns the raw `u32` an obj packs.
    #[inline(always)]
    pub const fn packed(&self) -> u32 {
        self.0
    }

    /// Returns the serial handed out after this one, skipping [`NONE`](Self::NONE).
    #[inline(always)]
    pub const fn next(self) -> Self {
        Self(self.0.wrapping_add(1) % Self::NONE.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_counts_up() {
        assert_eq!(ObjSerial::FIRST.next(), ObjSerial::from_packed(1));
    }

    #[test]
    fn next_skips_none() {
        let last = ObjSerial::from_packed(u32::MAX - 1);
        assert_eq!(last.next(), ObjSerial::FIRST);
        assert_ne!(last.next(), ObjSerial::NONE);
    }

    #[test]
    fn next_after_none_is_first() {
        assert_eq!(ObjSerial::NONE.next(), ObjSerial::FIRST);
    }

    #[test]
    fn packed_round_trips() {
        let serial = ObjSerial::from_packed(123_456);
        assert_eq!(ObjSerial::from_packed(serial.packed()), serial);
    }
}
