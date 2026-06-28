include!(concat!(env!("OUT_DIR"), "/level_xp.rs"));

/// A fixed-size collection of combat stats with current levels, base levels,
/// experience points, and change-tracking snapshots.
///
/// `N` is the number of stats: 21 for players, 6 for NPCs.
///
/// The `xp` array tracks cumulative experience per stat (used by players,
/// zeroed for NPCs). The `last_xp` and `last_levels` arrays track the
/// previous tick's values for client delta transmission.

#[derive(Debug, Clone)]
pub struct Stats<const N: usize> {
    pub levels: [u16; N],
    pub base_levels: [u16; N],
    pub xp: [i32; N],
    pub last_xp: [Option<i32>; N],
    pub last_levels: [Option<u16>; N],
}

impl<const N: usize> Stats<N> {
    /// Creates a new `StatBlock` with all levels set to `default_level`,
    /// zero experience, and no change-tracking history.
    #[inline(always)]
    pub const fn new(default_level: u16) -> Self {
        Self {
            levels: [default_level; N],
            base_levels: [default_level; N],
            xp: [0; N],
            last_xp: [None; N],
            last_levels: [None; N],
        }
    }

    /// Returns the current (boosted/drained) level for a stat.
    #[inline(always)]
    pub const fn level(&self, stat: usize) -> u16 {
        self.levels[stat]
    }

    /// Returns the base (unboosted) level for a stat.
    #[inline(always)]
    pub const fn base_level(&self, stat: usize) -> u16 {
        self.base_levels[stat]
    }

    /// Returns the number of stats. Always `N` (compile-time constant).
    #[allow(clippy::len_without_is_empty)]
    #[inline(always)]
    pub const fn len(&self) -> usize {
        N
    }

    /// Returns the sum of all base levels.
    pub fn total(&self) -> i32 {
        self.base_levels.iter().map(|&l| l as i32).sum()
    }

    /// Clamps a computed level into the storable `0..=65535` range and narrows
    /// it to a `u16`.
    #[inline(always)]
    fn clamp_level(value: i32) -> u16 {
        value.clamp(0, 65535) as u16
    }

    /// Raises the current level by a flat amount plus a percentage of the
    /// base level. The result is clamped to `[0, 65535]`.
    ///
    /// Formula: `current + (constant + base * percent / 100)`
    pub fn add(&mut self, stat: usize, constant: i32, percent: i32) {
        let base = self.base_levels[stat] as i32;
        let current = self.levels[stat] as i32;
        let added = current + (constant + base * percent / 100);
        self.levels[stat] = Self::clamp_level(added);
    }

    /// Lowers the current level by a flat amount plus a percentage of the
    /// base level, floored at 0.
    ///
    /// Formula: `current - (constant + base * percent / 100)`
    pub fn sub(&mut self, stat: usize, constant: i32, percent: i32) {
        let base = self.base_levels[stat] as i32;
        let current = self.levels[stat] as i32;
        let subbed = current - (constant + base * percent / 100);
        self.levels[stat] = subbed.max(0) as u16;
    }

    /// Restores the current level by a flat amount plus a percentage of the
    /// base level, capped at the base level. Will not lower the current
    /// level if it is already above base.
    ///
    /// Formula: `min(current + (constant + base * percent / 100), base)`
    pub fn heal(&mut self, stat: usize, constant: i32, percent: i32) {
        let base = self.base_levels[stat] as i32;
        let current = self.levels[stat] as i32;
        let healed = current + (constant + base * percent / 100);
        self.levels[stat] = healed.min(base).max(current) as u16;
    }

    /// Boosts the current level by a flat amount plus a percentage of the
    /// base level. The result is capped at `base + boost_amount` and will
    /// not lower the current level if it already exceeds that cap.
    ///
    /// Formula: `min(current + amount, base + amount)` where
    /// `amount = constant + base * percent / 100`
    pub fn boost(&mut self, stat: usize, constant: i32, percent: i32) {
        let base = self.base_levels[stat] as i32;
        let current = self.levels[stat] as i32;
        let amount = constant + base * percent / 100;
        let boosted = (current + amount).min(base + amount).max(current);
        self.levels[stat] = Self::clamp_level(boosted);
    }

    /// Drains the current level by a flat amount plus a percentage of the
    /// **current** level (not base), floored at 0.
    ///
    /// Formula: `current - (constant + current * percent / 100)`
    pub fn drain(&mut self, stat: usize, constant: i32, percent: i32) {
        let current = self.levels[stat] as i32;
        let drained = current - (constant + current * percent / 100);
        self.levels[stat] = drained.max(0) as u16;
    }

    /// Resets all current levels to their base levels.
    #[inline(always)]
    pub const fn reset(&mut self) {
        self.levels = self.base_levels;
    }

    /// Returns indices of stats whose xp or level changed since last
    /// snapshot. Updates the snapshots for each changed stat.
    pub fn collect_dirty(&mut self) -> impl Iterator<Item = usize> + '_ {
        (0..N).filter(|&i| {
            let dirty =
                self.last_xp[i] != Some(self.xp[i]) || self.last_levels[i] != Some(self.levels[i]);
            if dirty {
                self.last_xp[i] = Some(self.xp[i]);
                self.last_levels[i] = Some(self.levels[i]);
            }
            dirty
        })
    }

    /// Awards experience in a stat, capping at 2,000,000,000, and
    /// recalculates the base level from the experience curve. Experience is
    /// stored at 10× real XP (so the 200M cap becomes 2B), leaving room for
    /// fractional awards; the client divides by 10 for display.
    ///
    /// If the base level increases, the current level is adjusted:
    /// - If current equals the old base, it is set to the new base.
    /// - If current is below the old base, the difference is added.
    ///
    /// Returns `true` if the base level increased (the caller should
    /// trigger any level-up side effects).
    pub fn add_xp(&mut self, stat: usize, xp: i32) -> bool {
        if xp <= 0 {
            return false;
        }
        self.xp[stat] = self.xp[stat].saturating_add(xp).min(2_000_000_000);
        let before = self.base_levels[stat];
        let new_base = get_level_by_exp(self.xp[stat]) as u16;
        if self.levels[stat] == before {
            self.levels[stat] = new_base;
        } else if new_base > before && self.levels[stat] < before {
            self.levels[stat] += new_base - before;
        }
        self.base_levels[stat] = new_base;
        new_base > before
    }
}

/// Returns the level for a given experience total (stored at 10× real XP).
///
/// Binary search over the precomputed [`LEVEL_XP`] thresholds: the
/// level is the count of thresholds the experience has reached or passed
/// (clamped to a minimum of 1).
pub fn get_level_by_exp(exp: i32) -> u8 {
    LEVEL_XP
        .partition_point(|&threshold| threshold <= exp)
        .max(1) as u8
}

/// Returns the minimum experience (stored at 10× real XP) required to reach the
/// given level.
///
/// O(1) lookup in [`LEVEL_XP`]. Levels at or below 1 require 0; levels above 99
/// are clamped to the level-99 requirement.
///
/// # Arguments
/// * `level` - The target level (1..=99). Level 1 returns 0.
pub fn get_exp_by_level(level: u8) -> i32 {
    if level <= 1 {
        return 0;
    }
    LEVEL_XP[(level as usize - 1).min(LEVEL_XP.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_default_levels() {
        let stats: Stats<6> = Stats::new(1);
        for i in 0..6 {
            assert_eq!(stats.level(i), 1);
            assert_eq!(stats.base_level(i), 1);
            assert_eq!(stats.xp[i], 0);
            assert_eq!(stats.last_xp[i], None);
            assert_eq!(stats.last_levels[i], None);
        }
    }

    #[test]
    fn total() {
        let mut stats: Stats<4> = Stats::new(10);
        stats.base_levels[2] = 20;
        assert_eq!(stats.total(), 50);
    }

    #[test]
    fn add_clamps_at_max() {
        let mut stats: Stats<1> = Stats::new(99);
        stats.add(0, 100_000, 0);
        assert_eq!(stats.level(0), 65535);
    }

    #[test]
    fn sub_floors_at_zero() {
        let mut stats: Stats<1> = Stats::new(5);
        stats.sub(0, 10, 0);
        assert_eq!(stats.level(0), 0);
    }

    #[test]
    fn heal_caps_at_base() {
        let mut stats: Stats<1> = Stats::new(50);
        stats.levels[0] = 30;
        stats.heal(0, 100, 0);
        assert_eq!(stats.level(0), 50);
    }

    #[test]
    fn heal_does_not_lower() {
        let mut stats: Stats<1> = Stats::new(50);
        stats.levels[0] = 60;
        stats.heal(0, 5, 0);
        assert_eq!(stats.level(0), 60);
    }

    #[test]
    fn boost_caps_at_base_plus_amount() {
        let mut stats: Stats<1> = Stats::new(99);
        stats.boost(0, 5, 0);
        assert_eq!(stats.level(0), 104);
    }

    #[test]
    fn boost_does_not_lower() {
        let mut stats: Stats<1> = Stats::new(99);
        stats.levels[0] = 120;
        stats.boost(0, 5, 0);
        assert_eq!(stats.level(0), 120);
    }

    #[test]
    fn drain_uses_current_for_percent() {
        let mut stats: Stats<1> = Stats::new(100);
        stats.drain(0, 0, 50);
        assert_eq!(stats.level(0), 50);
    }

    #[test]
    fn add_with_percent() {
        let mut stats: Stats<1> = Stats::new(80);
        stats.levels[0] = 70;
        stats.add(0, 3, 10);
        assert_eq!(stats.level(0), 81);
    }

    #[test]
    fn reset_restores_base() {
        let mut stats: Stats<4> = Stats::new(50);
        stats.levels[0] = 30;
        stats.levels[2] = 70;
        stats.reset();
        assert_eq!(stats.levels, stats.base_levels);
    }

    #[test]
    fn level_by_exp_boundaries() {
        // XP is stored at 10× real XP, so the curve thresholds are ×10.
        assert_eq!(get_level_by_exp(0), 1);
        assert_eq!(get_level_by_exp(830), 2);
        assert_eq!(get_level_by_exp(130_344_310), 99);
        assert_eq!(get_level_by_exp(2_000_000_000), 99);
    }

    #[test]
    fn exp_level_roundtrip() {
        #[rustfmt::skip]
        const EXPECTED: [i32; 99] = [
            0, 83, 174, 276, 388, 512, 650, 801, 969, 1_154,
            1_358, 1_584, 1_833, 2_107, 2_411, 2_746, 3_115, 3_523, 3_973, 4_470,
            5_018, 5_624, 6_291, 7_028, 7_842, 8_740, 9_730, 10_824, 12_031, 13_363,
            14_833, 16_456, 18_247, 20_224, 22_406, 24_815, 27_473, 30_408, 33_648, 37_224,
            41_171, 45_529, 50_339, 55_649, 61_512, 67_983, 75_127, 83_014, 91_721, 101_333,
            111_945, 123_660, 136_594, 150_872, 166_636, 184_040, 203_254, 224_466, 247_886, 273_742,
            302_288, 333_804, 368_599, 407_015, 449_428, 496_254, 547_953, 605_032, 668_051, 737_627,
            814_445, 899_257, 992_895, 1_096_278, 1_210_421, 1_336_443, 1_475_581, 1_629_200, 1_798_808, 1_986_068,
            2_192_818, 2_421_087, 2_673_114, 2_951_373, 3_258_594, 3_597_792, 3_972_294, 4_385_776, 4_842_295, 5_346_332,
            5_902_831, 6_517_253, 7_195_629, 7_944_614, 8_771_558, 9_684_577, 10_692_629, 11_805_606, 13_034_431,
        ];
        for (i, &exp) in EXPECTED.iter().enumerate() {
            let level = (i + 1) as u8;
            assert_eq!(get_exp_by_level(level), exp * 10, "exp for level {level}");
            assert_eq!(get_level_by_exp(exp * 10), level, "level for exp {exp}");
        }
        for level in 2..=99u8 {
            assert_eq!(get_level_by_exp(get_exp_by_level(level)), level);
        }
    }
}
