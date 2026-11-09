/// A region's slot of region space and the generation of the region holding
/// it, so a uid kept past its region's freeing can't reach the slot's next
/// one. Scripts hold it as an int.
///
/// Bit layout: `(generation << 12) | slot`
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct RegionUid(u32);

impl RegionUid {
    #[inline(always)]
    pub const fn new(slot: u16, generation: u32) -> Self {
        Self((generation << 12) | slot as u32)
    }

    /// Rebuilds a uid from the int a script holds it as.
    #[inline(always)]
    pub const fn from_packed(packed: u32) -> Self {
        Self(packed)
    }

    /// Returns the raw `u32` a script holds as an int.
    #[inline(always)]
    pub const fn packed(&self) -> u32 {
        self.0
    }

    #[inline(always)]
    pub const fn slot(&self) -> u16 {
        (self.0 & 0xFFF) as u16
    }

    #[inline(always)]
    pub const fn generation(&self) -> u32 {
        self.0 >> 12
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_and_generation_round_trip() {
        let uid = RegionUid::new(2076, 0xFFFFF);
        assert_eq!(uid.slot(), 2076);
        assert_eq!(uid.generation(), 0xFFFFF);
        assert_eq!(RegionUid::from_packed(uid.packed()), uid);
        assert_ne!(uid, RegionUid::new(2076, 7));
    }
}
