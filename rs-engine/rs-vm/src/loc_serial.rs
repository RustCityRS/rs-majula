#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LocSerial(u16);

impl LocSerial {
    /// How many bits a serial takes in a packed loc.
    pub const BITS: u32 = 14;

    /// The first serial a zone hands out.
    pub const FIRST: Self = Self(0);

    const MASK: u16 = (1 << Self::BITS) - 1;

    /// Rebuilds a serial from the bits a loc packs it into.
    #[inline(always)]
    pub const fn from_packed(packed: u16) -> Self {
        Self(packed & Self::MASK)
    }

    /// Returns the raw bits a loc packs.
    #[inline(always)]
    pub const fn packed(&self) -> u16 {
        self.0
    }

    /// Returns the serial handed out after this one.
    #[inline(always)]
    pub const fn next(self) -> Self {
        Self((self.0 + 1) & Self::MASK)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_counts_up() {
        assert_eq!(LocSerial::FIRST.next(), LocSerial::from_packed(1));
    }

    #[test]
    fn next_wraps_to_first() {
        let last = LocSerial::from_packed((1 << LocSerial::BITS) - 1);
        assert_eq!(last.next(), LocSerial::FIRST);
    }

    #[test]
    fn from_packed_keeps_only_serial_bits() {
        assert_eq!(
            LocSerial::from_packed(1 << LocSerial::BITS),
            LocSerial::FIRST
        );
    }

    #[test]
    fn packed_round_trips() {
        let serial = LocSerial::from_packed(12_345);
        assert_eq!(LocSerial::from_packed(serial.packed()), serial);
    }
}
