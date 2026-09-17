/// Creates a bitmask consisting of the lowest `bits` bits set to 1.
///
/// When `bits` is 32 or greater, returns `-1` (all bits set) to avoid
/// undefined-behavior-equivalent shifts. Otherwise computes `(1 << bits) - 1`
/// using wrapping arithmetic.
///
/// # Arguments
///
/// * `bits` - The number of least-significant bits to set. Values >= 32 produce
///   an all-ones mask (`-1`).
///
/// # Returns
///
/// An `i32` with the lowest `bits` bits set to 1 and all higher bits cleared,
/// or `-1` if `bits >= 32`.
///
/// # Side Effects
///
/// None. This is a pure `const fn`.
///
/// # Panics
///
/// This function does not panic.
///
/// # Call Stack
///
/// **Called by:** [`setbit_range`], [`clearbit_range`], [`setbit_range_toint`]
/// (all within this module).
///
/// **Calls:** Nothing (leaf function).
#[inline(always)]
const fn make_mask(bits: i32) -> i32 {
    if bits >= 32 {
        -1
    } else {
        (1i32 << bits).wrapping_sub(1)
    }
}

/// Sets all bits in the inclusive range `[start..=end]` to 1, preserving all
/// other bits in `num`.
///
/// Implements the SETBIT_RANGE VM opcode. A mask of `(end - start + 1)` ones
/// is generated via [`make_mask`], shifted left by `start`, and OR'd into
/// `num`.
///
/// # Arguments
///
/// * `num` - The base integer whose bits are modified.
/// * `start` - The zero-indexed least-significant bit of the range (inclusive).
/// * `end` - The zero-indexed most-significant bit of the range (inclusive).
///   Must be >= `start`.
///
/// # Returns
///
/// A new `i32` equal to `num` with bits `[start..=end]` guaranteed to be 1.
///
/// # Side Effects
///
/// None. This is a pure `const fn`.
///
/// # Panics
///
/// This function does not panic. Out-of-range shifts are handled by
/// `wrapping_shl`.
///
/// # Call Stack
///
/// **Called by:** VM bitfield ops in `rs-vm/src/ops/number.rs`
/// (`SETBIT_RANGE` opcode), and [`setbit_range_toint`] in this module.
///
/// **Calls:** [`make_mask`].
#[inline(always)]
pub const fn setbit_range(num: i32, start: i32, end: i32) -> i32 {
    num | make_mask(end - start + 1).wrapping_shl(start as u32)
}

/// Clears all bits in the inclusive range `[start..=end]` to 0, preserving all
/// other bits in `num`.
///
/// Implements the CLEARBIT_RANGE VM opcode. A mask of `(end - start + 1)` ones
/// is generated via [`make_mask`], shifted left by `start`, inverted, and
/// AND'd with `num`.
///
/// # Arguments
///
/// * `num` - The base integer whose bits are modified.
/// * `start` - The zero-indexed least-significant bit of the range (inclusive).
/// * `end` - The zero-indexed most-significant bit of the range (inclusive).
///   Must be >= `start`.
///
/// # Returns
///
/// A new `i32` equal to `num` with bits `[start..=end]` guaranteed to be 0.
///
/// # Side Effects
///
/// None. This is a pure `const fn`.
///
/// # Panics
///
/// This function does not panic. Out-of-range shifts are handled by
/// `wrapping_shl`.
///
/// # Call Stack
///
/// **Called by:** VM bitfield ops in `rs-vm/src/ops/number.rs`
/// (`CLEARBIT_RANGE` opcode), and [`setbit_range_toint`] in this module.
///
/// **Calls:** [`make_mask`].
#[inline(always)]
pub const fn clearbit_range(num: i32, start: i32, end: i32) -> i32 {
    num & !make_mask(end - start + 1).wrapping_shl(start as u32)
}

/// Writes `value` into the bit range `[start..=end]` of `num`, clamping
/// `value` to the maximum representable by that range.
///
/// Implements the SETBIT_RANGE_TOINT VM opcode. The target bit range is first
/// cleared via [`clearbit_range`], then `value` (clamped to the range's
/// maximum, i.e. `make_mask(end - start + 1)`) is shifted into position and
/// OR'd in.
///
/// # Arguments
///
/// * `num` - The base integer whose bit range is overwritten.
/// * `value` - The value to store. If it exceeds the maximum that fits in the
///   `(end - start + 1)`-bit range, it is clamped to that maximum.
/// * `start` - The zero-indexed least-significant bit of the range (inclusive).
/// * `end` - The zero-indexed most-significant bit of the range (inclusive).
///   Must be >= `start`.
///
/// # Returns
///
/// A new `i32` equal to `num` with bits `[start..=end]` replaced by the
/// (possibly clamped) `value`.
///
/// # Side Effects
///
/// None. This is a pure `const fn`.
///
/// # Panics
///
/// This function does not panic.
///
/// # Call Stack
///
/// **Called by:** VM bitfield ops in `rs-vm/src/ops/number.rs`
/// (`SETBIT_RANGE_TOINT` opcode).
///
/// **Calls:** [`make_mask`], [`clearbit_range`].
#[inline(always)]
pub const fn setbit_range_toint(num: i32, value: i32, start: i32, end: i32) -> i32 {
    let max = make_mask(end - start + 1);
    let assign = if value > max { max } else { value };
    clearbit_range(num, start, end) | assign.wrapping_shl(start as u32)
}

#[cfg(test)]
mod tests {
    use crate::bits::{clearbit_range, setbit_range, setbit_range_toint};

    #[test]
    fn test_bitcount() {
        let count = 15_i32.count_ones();
        assert_eq!(count, 4);
    }

    #[test]
    fn test_setbit_range() {
        let result = setbit_range(0, 1, 3);
        assert_eq!(result, 14);
    }

    #[test]
    fn test_setbit_range_toint() {
        let result = setbit_range_toint(0, 3, 1, 3);
        assert_eq!(result, 6);
    }

    #[test]
    fn test_clearbit_range() {
        let result = clearbit_range(15, 1, 3);
        assert_eq!(result, 1);
    }

    #[test]
    fn setbit_range_single_bit() {
        assert_eq!(setbit_range(0, 0, 0), 1);
        assert_eq!(setbit_range(0, 5, 5), 32);
    }

    #[test]
    fn setbit_range_preserves_existing_bits() {
        let result = setbit_range(1, 4, 7);
        assert_eq!(result & 1, 1);
        assert_eq!(result & 0xF0, 0xF0);
    }

    #[test]
    fn clearbit_range_single_bit() {
        assert_eq!(clearbit_range(0xFF, 0, 0), 0xFE);
        assert_eq!(clearbit_range(0xFF, 7, 7), 0x7F);
    }

    #[test]
    fn clearbit_range_preserves_other_bits() {
        let result = clearbit_range(0xFF, 2, 5);
        assert_eq!(result, 0xFF & !0x3C);
    }

    #[test]
    fn setbit_range_toint_zero_value() {
        let result = setbit_range_toint(0xFF, 0, 4, 7);
        assert_eq!(result & 0xF0, 0);
        assert_eq!(result & 0x0F, 0x0F);
    }

    #[test]
    fn setbit_range_toint_max_value_clamped() {
        let result = setbit_range_toint(0, 255, 0, 3);
        assert_eq!(result, 15);
    }

    #[test]
    fn setbit_range_toint_exact_fit() {
        let result = setbit_range_toint(0, 7, 0, 2);
        assert_eq!(result, 7);
    }

    #[test]
    fn round_trip_set_clear() {
        let original = 0;
        let set = setbit_range(original, 3, 6);
        let cleared = clearbit_range(set, 3, 6);
        assert_eq!(cleared, original);
    }

    #[test]
    fn setbit_range_full_width() {
        let result = setbit_range(0, 0, 31);
        assert_eq!(result, -1i32);
    }

    #[test]
    fn clearbit_range_full_width() {
        let result = clearbit_range(-1i32, 0, 31);
        assert_eq!(result, 0);
    }

    #[test]
    fn setbit_range_toint_high_bits() {
        let result = setbit_range_toint(0, 3, 28, 31);
        assert_eq!((result >> 28) & 0xF, 3);
    }

    #[test]
    fn getbit_range_extraction() {
        // Mimics the GETBIT_RANGE opcode: ((a << (31-c)) as u32 >> (b + 31-c)) as i32
        let a = setbit_range_toint(0, 5, 4, 7); // value 5 at bits 4-7
        let b = 4; // start bit
        let c = 7; // end bit
        let r = 31 - c;
        let extracted = ((a.wrapping_shl(r as u32) as u32) >> ((b + r) as u32)) as i32;
        assert_eq!(extracted, 5);
    }

    #[test]
    fn getbit_range_from_zero() {
        let a = setbit_range_toint(0, 3, 0, 2);
        let b = 0;
        let c = 2;
        let r = 31 - c;
        let extracted = ((a.wrapping_shl(r as u32) as u32) >> ((b + r) as u32)) as i32;
        assert_eq!(extracted, 3);
    }

    #[test]
    fn setbit_and_testbit_pattern() {
        // Mimics SETBIT: a | (1 << b), TESTBIT: (a & (1 << b)) != 0
        let mut val = 0i32;
        val |= 1i32 << 5; // setbit bit 5
        assert_ne!(val & (1i32 << 5), 0); // testbit bit 5
        assert_eq!(val & (1i32 << 3), 0); // testbit bit 3
    }

    #[test]
    fn clearbit_pattern() {
        // Mimics CLEARBIT: a & !(1 << b)
        let val = 0xFFi32;
        let cleared = val & !(1i32 << 3);
        assert_eq!(cleared, 0xF7);
    }

    #[test]
    fn togglebit_pattern() {
        // Mimics TOGGLEBIT: a ^ (1 << b)
        let val = 0i32;
        let toggled = val ^ (1 << 5);
        assert_eq!(toggled, 32);
        let toggled_back = toggled ^ (1 << 5);
        assert_eq!(toggled_back, 0);
    }

    #[test]
    fn clearbit_range_reversed_args_as_in_opcode() {
        // The CLEARBIT_RANGE opcode pops c, b, a and calls clearbit_range(c, b, a)
        // i.e., the number to clear FROM is the third popped value
        let num = 0xFF;
        let start = 2;
        let end = 5;
        let result = clearbit_range(num, start, end);
        // Clears bits 2-5 from 0xFF, leaving bits 0-1 and 6-7
        assert_eq!(result, 0xFF & !(0xF << 2));
    }

    #[test]
    fn setbit_range_toint_round_trip_with_getbit_range() {
        // Set value, then extract it back
        for value in [0, 1, 7, 15] {
            let packed = setbit_range_toint(0, value, 8, 11);
            let r = 31 - 11;
            let extracted = ((packed.wrapping_shl(r as u32) as u32) >> ((8 + r) as u32)) as i32;
            assert_eq!(extracted, value, "round-trip failed for value {value}");
        }
    }

    #[test]
    fn bitcount_pattern() {
        // Mimics BITCOUNT opcode: a.count_ones()
        assert_eq!(0i32.count_ones(), 0);
        assert_eq!(0xFFi32.count_ones(), 8);
        assert_eq!(0x5555i32.count_ones(), 8);
        assert_eq!((-1i32).count_ones(), 32);
    }
}
