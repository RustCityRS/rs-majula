/// A controller's slot and the generation of the controller holding it, so a
/// uid kept past its controller's deletion can't reach the slot's next one.
///
/// Bit layout: `(generation << 16) | slot`
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ControllerUid(u32);

impl ControllerUid {
    #[inline(always)]
    pub const fn new(slot: u16, generation: u16) -> Self {
        Self(((generation as u32) << 16) | slot as u32)
    }

    #[inline(always)]
    pub const fn slot(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }

    #[inline(always)]
    pub const fn generation(&self) -> u16 {
        (self.0 >> 16) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_and_generation_round_trip() {
        let uid = ControllerUid::new(513, 7);
        assert_eq!(uid.slot(), 513);
        assert_eq!(uid.generation(), 7);
        assert_ne!(uid, ControllerUid::new(513, 8));
    }
}
