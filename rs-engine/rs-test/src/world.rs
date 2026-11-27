//! Placing a test's player around the world's npcs, which wander between ticks and runs.

use rs_engine::Engine;
use rsmod::rsmod::collision::collision_strategy::CollisionType;

/// A walkable tile beside (x, z) that (x, z) itself can step to: a walkable tile can be across a
/// wall or fence from it.
pub fn beside(level: u8, x: u16, z: u16) -> Option<(u16, u16)> {
    [
        (1i8, 0i8),
        (-1, 0),
        (0, 1),
        (0, -1),
        (1, 1),
        (-1, -1),
        (1, -1),
        (-1, 1),
    ]
    .into_iter()
    .filter(|&(dx, dz)| rsmod::can_travel(level, x, z, dx, dz, 1, 0, CollisionType::Normal))
    .map(|(dx, dz)| {
        (
            x.wrapping_add_signed(dx as i16),
            z.wrapping_add_signed(dz as i16),
        )
    })
    .find(|&(tx, tz)| !rsmod::is_flagged(tx, tz, level, 0x100 | 0x200000))
}

/// Stops every npc within `radius` tiles of (x, z) hunting: an aggressive npc's swing redirects
/// the player's op through auto-retaliate, or kills them, on ticks its wander rolls decide.
pub fn calm(engine: &mut Engine, x: u16, z: u16, radius: u16) {
    for active in engine.npc_list.npcs.iter_mut().flatten() {
        let c = active.npc.pathing.coord;
        if c.x().abs_diff(x) <= radius && c.z().abs_diff(z) <= radius {
            active.npc.hunt_range = 0;
        }
    }
}
