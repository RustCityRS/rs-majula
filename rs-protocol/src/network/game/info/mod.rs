#[cfg(since_647)]
pub mod hit;
pub mod npc;
pub mod player;

/// A full health bar.
#[cfg(since_530)]
const FULL: u32 = 255;

/// The health bar's fill, 255 for full: `current` of `base`.
#[cfg(since_530)]
pub fn health(current: u8, base: u8) -> u8 {
    if base == 0 {
        return 0;
    }
    (current as u32 * FULL / base as u32).min(FULL) as u8
}

/// The bytes `psmartint2` takes for `value`.
#[cfg(since_742)]
const fn smart2or4null_size(value: i32) -> usize {
    if value >= 32767 { 4 } else { 2 }
}
