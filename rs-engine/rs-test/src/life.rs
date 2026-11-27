//! A player's life: `%life_points` (varp 1240 bits 1-15, ten to a hitpoint) from 647, the
//! hitpoints stat before.

use rs_engine::Engine;
use rs_entity::Player;
#[cfg(since_647)]
use rs_pack::cache::VarValue;
#[cfg(before_647)]
use rs_pack::types::PlayerStat;

/// Life per hitpoint.
#[cfg(since_647)]
pub const SCALE: i32 = 10;
#[cfg(before_647)]
pub const SCALE: i32 = 1;

#[cfg(since_647)]
const VARP: u16 = 1240;

pub fn of(player: &Player) -> i32 {
    #[cfg(since_647)]
    return (player.vars.get(VARP).as_int() >> 1) & 0x7FFF;
    #[cfg(before_647)]
    return player.stats.levels[PlayerStat::Hitpoints as usize] as i32;
}

pub fn get(engine: &Engine, pid: u16) -> i32 {
    of(engine.harness_player(pid).expect("player"))
}

pub fn set(player: &mut Player, life: i32) {
    #[cfg(since_647)]
    {
        let bit0 = player.vars.get(VARP).as_int() & 1;
        player.vars.set(VARP, VarValue::Int((life << 1) | bit0));
    }
    #[cfg(before_647)]
    {
        player.stats.levels[PlayerStat::Hitpoints as usize] = life as _;
    }
}
