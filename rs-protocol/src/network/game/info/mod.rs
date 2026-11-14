#[cfg(since_647)]
pub mod hit;
pub mod npc;
pub mod player;

/// The bytes `psmart2or4null` takes for `value`.
#[cfg(since_742)]
const fn smart2or4null_size(value: i32) -> usize {
    if value >= 32767 { 4 } else { 2 }
}
