// 48-bit linear congruential pseudorandom number generator as specified by
// java.util.Random (J2SE 1.2).

/// The multiplier constant used in the linear congruential formula.
///
/// This is the value `0x5DEECE66D` (25214903917 in decimal), taken directly from the
/// Java `java.util.Random` specification (J2SE 1.2). It is applied to the internal seed
/// on every call to [`JavaRandom::next`] via `seed = (seed * MULTIPLIER + ADDEND) & MASK`.
const MULTIPLIER: i64 = 0x5DEECE66D;

/// The additive constant (increment) used in the linear congruential formula.
///
/// This is the value `0xB` (11 in decimal), taken directly from the Java `java.util.Random`
/// specification (J2SE 1.2). It is added to the product of the seed and [`MULTIPLIER`] on
/// every call to [`JavaRandom::next`].
const ADDEND: i64 = 0xB;

/// Bit mask that truncates the internal seed to 48 bits after each LCG step.
///
/// Equal to `(1 << 48) - 1` (`0xFFFF_FFFF_FFFF`). Applied via bitwise AND to ensure the
/// seed never exceeds 48 bits, matching the behavior of `java.util.Random`.
const MASK: i64 = (1i64 << 48) - 1;

/// A 48-bit linear congruential pseudorandom number generator (PRNG) that exactly
/// reproduces the output of `java.util.Random` (J2SE 1.2).
///
/// This struct maintains the 48-bit internal seed and a one-element cache for the
/// Box-Muller gaussian transform (see [`JavaRandom::next_gaussian`]). Given the same
/// initial seed, every method in this implementation produces the identical sequence
/// of values as the corresponding Java methods, which is required for deterministic
/// engine replay.
///
/// # Usage
///
/// `JavaRandom` is used by `Engine` (`rs-engine/src/engine.rs`) for all deterministic
/// random number generation and is exposed to scripts via the `ScriptEngine` trait.
pub struct JavaRandom {
    seed: i64,
    next_next_gaussian: f64,
    have_next_next_gaussian: bool,
}

impl JavaRandom {
    /// Creates a new `JavaRandom` instance initialized with the given seed.
    ///
    /// The seed is scrambled through [`JavaRandom::set_seed`] before first use, exactly
    /// as `java.util.Random(long seed)` does. Two instances created with the same seed
    /// will produce identical sequences of random values across all methods.
    ///
    /// # Arguments
    ///
    /// * `seed` - The initial seed value. Any `i64` is valid; it will be XOR-ed with
    ///   [`MULTIPLIER`] and masked to 48 bits internally.
    ///
    /// # Returns
    ///
    /// A new `JavaRandom` whose internal state is ready to produce random values.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `Engine` (rs-engine/src/engine.rs), `ScriptEngine` trait implementations.
    /// **Calls:** [`JavaRandom::set_seed`].
    pub fn new(seed: i64) -> Self {
        let mut rng = Self {
            seed: 0,
            next_next_gaussian: 0.0,
            have_next_next_gaussian: false,
        };
        rng.set_seed(seed);
        rng
    }

    /// Resets the generator to a new seed, discarding all previous state.
    ///
    /// The raw seed is XOR-ed with [`MULTIPLIER`] and masked to 48 bits, matching the
    /// behavior of `java.util.Random.setSeed(long)`. Any cached gaussian value from a
    /// prior [`JavaRandom::next_gaussian`] call is discarded.
    ///
    /// # Arguments
    ///
    /// * `seed` - The new seed value. Any `i64` is valid.
    ///
    /// # Side Effects
    ///
    /// * Overwrites the internal 48-bit seed.
    /// * Sets `have_next_next_gaussian` to `false`, clearing the gaussian cache.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`JavaRandom::new`], engine reset paths.
    /// **Calls:** Nothing.
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ MULTIPLIER) & MASK;
        self.have_next_next_gaussian = false;
    }

    /// Advances the internal seed by one LCG step and returns the top `bits` bits.
    ///
    /// This is the core primitive of the generator, equivalent to `java.util.Random.next(int)`.
    /// It computes `seed = (seed * MULTIPLIER + ADDEND) & MASK`, then right-shifts the new
    /// 48-bit seed to extract the requested number of high-order bits. All public generation
    /// methods delegate to this function.
    ///
    /// # Arguments
    ///
    /// * `bits` - The number of pseudorandom bits to produce (1..=32). Values outside this
    ///   range will still work but produce fewer meaningful bits.
    ///
    /// # Returns
    ///
    /// An `i32` whose lower `bits` bits are pseudorandom and whose upper bits are zero.
    ///
    /// # Side Effects
    ///
    /// * Mutates the internal seed to the next value in the LCG sequence.
    ///
    /// # Call Stack
    ///
    /// **Called by:** [`JavaRandom::next_int`], [`JavaRandom::next_int_bound`],
    /// [`JavaRandom::next_long`], [`JavaRandom::next_boolean`], [`JavaRandom::next_float`],
    /// [`JavaRandom::next_double`], [`JavaRandom::next_bytes`].
    /// **Calls:** Nothing.
    fn next(&mut self, bits: i32) -> i32 {
        let next_seed = (self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
        self.seed = next_seed;
        ((next_seed as u64) >> (48 - bits)) as i32
    }

    /// Returns a uniformly distributed pseudorandom 32-bit integer.
    ///
    /// Equivalent to `java.util.Random.nextInt()`. All 2^32 possible `i32` values are
    /// produced with approximately equal probability.
    ///
    /// # Returns
    ///
    /// A pseudorandom `i32` spanning the full 32-bit range.
    ///
    /// # Side Effects
    ///
    /// * Advances the internal seed by one LCG step.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Engine game-logic paths, [`JavaRandom::next_bytes`].
    /// **Calls:** [`JavaRandom::next`] with `bits = 32`.
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }

    /// Returns a uniformly distributed pseudorandom integer in the range `[0, n)`.
    ///
    /// Equivalent to `java.util.Random.nextInt(int bound)`. When `n` is a power of two the
    /// result is computed with a single call to [`JavaRandom::next`] for maximum efficiency.
    /// Otherwise a rejection-sampling loop is used to eliminate modulo bias, which may
    /// require multiple calls to [`JavaRandom::next`].
    ///
    /// # Arguments
    ///
    /// * `n` - The exclusive upper bound. Must be positive (`n > 0`).
    ///
    /// # Returns
    ///
    /// A pseudorandom `i32` in the half-open range `[0, n)`.
    ///
    /// # Panics
    ///
    /// Panics with `"n must be positive"` if `n <= 0`.
    ///
    /// # Side Effects
    ///
    /// * Advances the internal seed by one or more LCG steps (one when `n` is a power of
    ///   two; potentially more otherwise due to rejection sampling).
    ///
    /// # Call Stack
    ///
    /// **Called by:** Engine game-logic paths (bounded random rolls).
    /// **Calls:** [`JavaRandom::next`] with `bits = 31`.
    pub fn next_int_bound(&mut self, n: i32) -> i32 {
        assert!(n > 0, "n must be positive");

        if (n & -n) == n {
            return ((n as i64).wrapping_mul(self.next(31) as i64) >> 31) as i32;
        }

        let mut bits;
        let mut val;
        loop {
            bits = self.next(31);
            val = bits % n;
            if bits - val + (n - 1) >= 0 {
                break;
            }
        }
        val
    }

    /// Returns a uniformly distributed pseudorandom 64-bit integer.
    ///
    /// Equivalent to `java.util.Random.nextLong()`. The result is formed by concatenating
    /// two 32-bit values: the upper 32 bits come from one call to [`JavaRandom::next`] and
    /// the lower 32 bits from a second call.
    ///
    /// # Returns
    ///
    /// A pseudorandom `i64` spanning the full 64-bit range.
    ///
    /// # Side Effects
    ///
    /// * Advances the internal seed by two LCG steps.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Engine game-logic paths.
    /// **Calls:** [`JavaRandom::next`] with `bits = 32` (twice).
    pub fn next_long(&mut self) -> i64 {
        ((self.next(32) as i64) << 32).wrapping_add(self.next(32) as i64)
    }

    /// Returns a pseudorandom boolean value.
    ///
    /// Equivalent to `java.util.Random.nextBoolean()`. Returns `true` or `false` with
    /// approximately equal probability.
    ///
    /// # Returns
    ///
    /// `true` or `false`, each with roughly 50% probability.
    ///
    /// # Side Effects
    ///
    /// * Advances the internal seed by one LCG step.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Engine game-logic paths.
    /// **Calls:** [`JavaRandom::next`] with `bits = 1`.
    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    /// Returns a pseudorandom `f32` uniformly distributed in `[0.0, 1.0)`.
    ///
    /// Equivalent to `java.util.Random.nextFloat()`. The value is computed by generating
    /// 24 random bits and dividing by 2^24, yielding a float with 24 bits of mantissa
    /// precision.
    ///
    /// # Returns
    ///
    /// A pseudorandom `f32` in the half-open range `[0.0, 1.0)`.
    ///
    /// # Side Effects
    ///
    /// * Advances the internal seed by one LCG step.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Engine game-logic paths.
    /// **Calls:** [`JavaRandom::next`] with `bits = 24`.
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1i32 << 24) as f32
    }

    /// Returns a pseudorandom `f64` uniformly distributed in `[0.0, 1.0)`.
    ///
    /// Equivalent to `java.util.Random.nextDouble()`. The value is computed by generating
    /// 53 random bits (26 bits shifted left by 27, plus another 27 bits) and dividing by
    /// 2^53, yielding a double with 53 bits of mantissa precision.
    ///
    /// # Returns
    ///
    /// A pseudorandom `f64` in the half-open range `[0.0, 1.0)`.
    ///
    /// # Side Effects
    ///
    /// * Advances the internal seed by two LCG steps.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Engine game-logic paths, [`JavaRandom::next_gaussian`].
    /// **Calls:** [`JavaRandom::next`] with `bits = 26` and `bits = 27`.
    pub fn next_double(&mut self) -> f64 {
        let l = ((self.next(26) as i64) << 27) + self.next(27) as i64;
        l as f64 / (1i64 << 53) as f64
    }

    /// Fills the given byte slice with pseudorandom bytes.
    ///
    /// Equivalent to `java.util.Random.nextBytes(byte[])`. For every four bytes, one 32-bit
    /// integer is generated via [`JavaRandom::next`] and its successive bytes (least
    /// significant first) are written into the slice. If the slice length is not a multiple
    /// of four, the final generated integer is only partially consumed.
    ///
    /// # Arguments
    ///
    /// * `bytes` - A mutable byte slice to fill. May be any length, including zero.
    ///
    /// # Side Effects
    ///
    /// * Advances the internal seed by `ceil(bytes.len() / 4)` LCG steps.
    /// * Overwrites every element of `bytes`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Engine game-logic paths.
    /// **Calls:** [`JavaRandom::next`] with `bits = 32`.
    pub fn next_bytes(&mut self, bytes: &mut [u8]) {
        let num_requested = bytes.len();
        let mut num_got = 0;
        let mut rnd = 0i32;
        loop {
            for i in 0..4 {
                if num_got == num_requested {
                    return;
                }
                rnd = if i == 0 { self.next(32) } else { rnd >> 8 };
                bytes[num_got] = rnd as u8;
                num_got += 1;
            }
        }
    }

    /// Returns a pseudorandom `f64` from a Gaussian (normal) distribution with mean 0.0
    /// and standard deviation 1.0.
    ///
    /// Equivalent to `java.util.Random.nextGaussian()`. Uses the polar form of the
    /// Box-Muller transform, which produces two independent gaussian values per iteration.
    /// The second value is cached in `next_next_gaussian` and returned on the following
    /// call, so roughly half the calls return immediately without generating new random
    /// doubles.
    ///
    /// # Returns
    ///
    /// A pseudorandom `f64` drawn from the standard normal distribution (mean = 0,
    /// std dev = 1).
    ///
    /// # Side Effects
    ///
    /// * On a generating call: advances the internal seed by multiple LCG steps (at least
    ///   four, possibly more due to rejection sampling) and stores the second gaussian
    ///   value in the cache.
    /// * On a cached call: clears the gaussian cache without advancing the seed.
    ///
    /// # Call Stack
    ///
    /// **Called by:** Engine game-logic paths.
    /// **Calls:** [`JavaRandom::next_double`] (multiple times per generating call).
    pub fn next_gaussian(&mut self) -> f64 {
        if self.have_next_next_gaussian {
            self.have_next_next_gaussian = false;
            return self.next_next_gaussian;
        }
        loop {
            let v1 = 2.0 * self.next_double() - 1.0;
            let v2 = 2.0 * self.next_double() - 1.0;
            let s = v1 * v1 + v2 * v2;
            if s < 1.0 && s != 0.0 {
                let m = (-2.0 * s.ln() / s).sqrt();
                self.next_next_gaussian = v2 * m;
                self.have_next_next_gaussian = true;
                return v1 * m;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_zero_next_int() {
        let mut rng = JavaRandom::new(0);
        assert_eq!(rng.next_int(), -1155484576);
        assert_eq!(rng.next_int(), -723955400);
        assert_eq!(rng.next_int(), 1033096058);
        assert_eq!(rng.next_int(), -1690734402);
        assert_eq!(rng.next_int(), -1557280266);
    }

    #[test]
    fn seed_12345_next_int() {
        let mut rng = JavaRandom::new(12345);
        assert_eq!(rng.next_int(), 1553932502);
        assert_eq!(rng.next_int(), -2090749135);
        assert_eq!(rng.next_int(), -287790814);
        assert_eq!(rng.next_int(), -355989640);
        assert_eq!(rng.next_int(), -716867186);
    }

    #[test]
    fn negative_seed_next_int() {
        let mut rng = JavaRandom::new(-1);
        assert_eq!(rng.next_int(), 1155099827);
    }

    #[test]
    fn seed_zero_next_long() {
        let mut rng = JavaRandom::new(0);
        assert_eq!(rng.next_long(), -4962768465676381896);
        assert_eq!(rng.next_long(), 4437113781045784766);
        assert_eq!(rng.next_long(), -6688467811848818630);
    }

    #[test]
    fn next_int_bound_deterministic() {
        let mut a = JavaRandom::new(0);
        let mut b = JavaRandom::new(0);
        for n in [1, 2, 3, 5, 7, 10, 16, 100, 1000, i32::MAX] {
            assert_eq!(a.next_int_bound(n), b.next_int_bound(n));
        }
    }

    #[test]
    fn next_int_bound_range() {
        let mut rng = JavaRandom::new(42);
        for n in [1, 2, 3, 5, 10, 16, 100, 1000000] {
            for _ in 0..100 {
                let val = rng.next_int_bound(n);
                assert!(val >= 0 && val < n, "val={val} n={n}");
            }
        }
    }

    #[test]
    fn next_int_bound_one_always_zero() {
        let mut rng = JavaRandom::new(0);
        for _ in 0..100 {
            assert_eq!(rng.next_int_bound(1), 0);
        }
    }

    #[test]
    #[should_panic(expected = "n must be positive")]
    fn next_int_bound_zero_panics() {
        JavaRandom::new(0).next_int_bound(0);
    }

    #[test]
    #[should_panic(expected = "n must be positive")]
    fn next_int_bound_negative_panics() {
        JavaRandom::new(0).next_int_bound(-5);
    }

    #[test]
    fn next_boolean_deterministic() {
        let mut a = JavaRandom::new(0);
        let mut b = JavaRandom::new(0);
        for _ in 0..50 {
            assert_eq!(a.next_boolean(), b.next_boolean());
        }
    }

    #[test]
    fn next_float_range() {
        let mut rng = JavaRandom::new(0);
        for _ in 0..1000 {
            let f = rng.next_float();
            assert!((0.0..1.0).contains(&f));
        }
    }

    #[test]
    fn next_double_range() {
        let mut rng = JavaRandom::new(0);
        for _ in 0..1000 {
            let d = rng.next_double();
            assert!((0.0..1.0).contains(&d));
        }
    }

    #[test]
    fn next_float_deterministic() {
        let mut a = JavaRandom::new(0);
        let mut b = JavaRandom::new(0);
        for _ in 0..50 {
            assert_eq!(a.next_float().to_bits(), b.next_float().to_bits());
        }
    }

    #[test]
    fn next_double_deterministic() {
        let mut a = JavaRandom::new(0);
        let mut b = JavaRandom::new(0);
        for _ in 0..50 {
            assert_eq!(a.next_double().to_bits(), b.next_double().to_bits());
        }
    }

    #[test]
    fn next_bytes_deterministic() {
        let mut a = JavaRandom::new(0);
        let mut b = JavaRandom::new(0);
        let mut buf_a = [0u8; 32];
        let mut buf_b = [0u8; 32];
        a.next_bytes(&mut buf_a);
        b.next_bytes(&mut buf_b);
        assert_eq!(buf_a, buf_b);
    }

    #[test]
    fn next_bytes_matches_next_int() {
        let mut rng_bytes = JavaRandom::new(0);
        let mut rng_ints = JavaRandom::new(0);
        let mut buf = [0u8; 8];
        rng_bytes.next_bytes(&mut buf);

        let i1 = rng_ints.next_int();
        assert_eq!(buf[0], i1 as u8);
        assert_eq!(buf[1], (i1 >> 8) as u8);
        assert_eq!(buf[2], (i1 >> 16) as u8);
        assert_eq!(buf[3], (i1 >> 24) as u8);

        let i2 = rng_ints.next_int();
        assert_eq!(buf[4], i2 as u8);
        assert_eq!(buf[5], (i2 >> 8) as u8);
        assert_eq!(buf[6], (i2 >> 16) as u8);
        assert_eq!(buf[7], (i2 >> 24) as u8);
    }

    #[test]
    fn next_gaussian_deterministic() {
        let mut a = JavaRandom::new(0);
        let mut b = JavaRandom::new(0);
        for _ in 0..20 {
            assert_eq!(a.next_gaussian().to_bits(), b.next_gaussian().to_bits());
        }
    }

    #[test]
    fn set_seed_resets_state() {
        let mut rng = JavaRandom::new(42);
        let v1 = rng.next_int();
        let v2 = rng.next_int();
        rng.set_seed(42);
        assert_eq!(rng.next_int(), v1);
        assert_eq!(rng.next_int(), v2);
    }

    #[test]
    fn set_seed_clears_gaussian() {
        let mut rng = JavaRandom::new(0);
        let _ = rng.next_gaussian();
        assert!(rng.have_next_next_gaussian);
        rng.set_seed(0);
        assert!(!rng.have_next_next_gaussian);
    }

    #[test]
    fn two_instances_same_seed_identical() {
        let mut a = JavaRandom::new(999);
        let mut b = JavaRandom::new(999);
        for _ in 0..50 {
            assert_eq!(a.next_int(), b.next_int());
        }
    }
}
