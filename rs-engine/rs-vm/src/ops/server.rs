use crate::engine::{ScriptEngine, cache, engine, engine_mut};
use crate::register::OpsRegistry;
use crate::state::ExecutionState;
#[cfg(since_274)]
use crate::util::midi_tick_length;
use crate::util::{pop_coord, pop_seq, pop_spotanim};
use crate::{handlers, none};
use rs_grid::CoordGrid;
use rs_pack::cache::script::*;

/// Registers server and world-level opcodes for coordinate utilities, map queries,
/// pathfinding checks, projectile animations, and zone management.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Coordinate helpers:** `COORDX`, `COORDY`, `COORDZ`, `MOVECOORD`, `DISTANCE`
/// - **Zone / area tests:** `INZONE`, `MAP_BLOCKED`, `MAP_INDOORS`, `MAP_MULTIWAY`,
///   `MAP_PLAYERCOUNT`, `MAP_MEMBERS`, `MAP_LIVE`, `MAP_LOCADDUNSAFE`
/// - **Pathfinding:** `LINEOFSIGHT`, `LINEOFWALK`, `MAP_FINDSQUARE`
/// - **World state:** `MAP_CLOCK`, `PLAYERCOUNT`
/// - **Animations / effects:** `PROJANIM_MAP`, `SPOTANIM_MAP`, `SEQLENGTH`
/// - **Suspension:** `WORLD_DELAY`
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::server::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 1000
        none!(m, COORDX => |s| {
            let coord = pop_coord(s)?;
            s.push_int(coord.x() as i32);
        });

        // 1001
        none!(m, COORDY => |s| {
            let coord = pop_coord(s)?;
            s.push_int(coord.y() as i32);
        });

        // 1002
        none!(m, COORDZ => |s| {
            let coord = pop_coord(s)?;
            s.push_int(coord.z() as i32);
        });

        // 1003
        none!(m, DISTANCE => |s| {
            let b = pop_coord(s)?;
            let a = pop_coord(s)?;
            s.push_int(a.distance(b));
        });

        // 1004
        none!(m, INZONE => |s| {
            let test = pop_coord(s)?;
            let ne = pop_coord(s)?;
            let sw = pop_coord(s)?;
            let ok = test.y() == sw.y()
                && test.x() >= sw.x()
                && test.x() <= ne.x()
                && test.z() >= sw.z()
                && test.z() <= ne.z();
            s.push_int(ok as i32);
        });

        // 1005
        none!(m, LINEOFSIGHT => |s| {
            let dst = pop_coord(s)?;
            let src = pop_coord(s)?;
            s.push_int(engine::<E>().lineofsight(src, dst)? as i32);
        });

        // 1006
        none!(m, LINEOFWALK => |s| {
            let dst = pop_coord(s)?;
            let src = pop_coord(s)?;
            s.push_int(engine::<E>().lineofwalk(src, dst)? as i32);
        });

        // 1007
        none!(m, MAP_BLOCKED => |s| {
            let coord = pop_coord(s)?;
            s.push_int(engine::<E>().map_blocked(coord)? as i32);
        });

        // 1008
        none!(m, MAP_CLOCK => |s| {
            s.push_int(engine::<E>().clock() as i32);
        });

        // 1009
        // Picks a random walkable tile in the square ring [minRadius, maxRadius] (Chebyshev distance)
        // around `coord`. `type` controls reachability: NONE = any open tile, LINEOFWALK / LINEOFSIGHT =
        // the tile must also have a clear walk/sight path back to the origin. Returns the input coord
        // unchanged when no tile qualifies (caller treats "result === coord" as "no square found").
        none!(m, MAP_FINDSQUARE => |s| {
            let find_type = s.pop_int();
            let max_radius = s.pop_int();
            let min_radius = s.pop_int();
            let coord = pop_coord(s)?;

            let engine = engine::<E>();
            let cache = cache();
            let not_members = !engine.members();
            let (cx, cz) = (coord.x() as i32, coord.z() as i32);

            // The reachability gate for a candidate tile back to the origin. Checked last in the loop
            // because line-of-walk/sight tracing is far more expensive than the other filters, so we
            // only pay for it on tiles that already passed the cheap checks.
            let passes_type = |src: CoordGrid| match find_type {
                1 => engine.lineofwalk(src, coord).unwrap_or(false),
                2 => engine.lineofsight(src, coord).unwrap_or(false),
                _ => true,
            };

            const MAX_TILES: usize = 100;
            let eligible: Vec<CoordGrid> = (cx - max_radius..=cx + max_radius)
                // Restrict the bounding box to the ring: skip the inner hole and anything past maxRadius.
                .flat_map(|x| (cz - max_radius..=cz + max_radius).map(move |z| (x, z)))
                .filter(|&(x, z)| {
                    let distance = (x - cx).abs().max((z - cz).abs());
                    (min_radius..=max_radius).contains(&distance)
                })
                // F2P node: discard members-only tiles.
                .filter(|&(x, z)| !not_members || cache.is_free(x as u16, z as u16))
                .map(|(x, z)| CoordGrid::new(x as u16, coord.y(), z as u16))
                // Must be a standable tile (no collision).
                // Finally the (costly) reachability requirement for this `type`.
                .filter(|&src| !engine.map_blocked(src).unwrap_or(true) && passes_type(src))
                .take(MAX_TILES)
                .collect();

            // No qualifying tile: hand back the original coord so the caller can detect the failure.
            if eligible.is_empty() {
                s.push_int(coord.packed() as i32);
            // Uniform roll among the collected candidates.
            } else {
                let index = (engine_mut::<E>().random().next_double() * eligible.len() as f64) as usize;
                s.push_int(eligible[index].packed() as i32);
            }
        });

        // 1010
        none!(m, MAP_INDOORS => |s| {
            let coord = pop_coord(s)?;
            s.push_int(engine::<E>().map_indoors(coord)? as i32);
        });

        // 1011
        none!(m, MAP_LIVE => |s| {
            #[cfg(debug_assertions)]
            s.push_int(0);
            #[cfg(not(debug_assertions))]
            s.push_int(1);
        });

        // 1012
        none!(m, MAP_LOCADDUNSAFE => |s| {
            let coord = pop_coord(s)?;
            s.push_int(engine::<E>().locaddunsafe(coord) as i32);
        });

        // 1013
        none!(m, MAP_MEMBERS => |s| {
            s.push_int(engine::<E>().members() as i32);
        });

        // 1014
        none!(m, MAP_MULTIWAY => |s| {
            let coord = pop_coord(s)?;
            s.push_int(cache().is_multi(coord.x(), coord.z(), coord.y()) as i32);
        });

        // 1015
        none!(m, MAP_PLAYERCOUNT => |s| {
            let to = pop_coord(s)?;
            let from = pop_coord(s)?;

            let mut count = 0;
            let from_zx = from.x() >> 3;
            let from_zz = from.z() >> 3;
            let to_zx = (to.x() + 7) >> 3;
            let to_zz = (to.z() + 7) >> 3;

            let engine = engine::<E>();

            for zx in from_zx..=to_zx {
                for zz in from_zz..=to_zz {
                    for coord in engine.get_zone_player_coords(CoordGrid::new(zx << 3, from.y(), zz << 3)) {
                        if coord.x() >= from.x() && coord.x() <= to.x() && coord.z() >= from.z() && coord.z() <= to.z() {
                            count += 1;
                        }
                    }
                }
            }

            s.push_int(count);
        });

        // 1016
        none!(m, MOVECOORD => |s| {
            let dz = s.pop_int();
            let dy = s.pop_int();
            let dx = s.pop_int();
            let base = pop_coord(s)?;
            let nc = CoordGrid::new(
                (base.x() as i32 + dx) as u16,
                (base.y() as i32 + dy).clamp(0, 3) as u8,
                (base.z() as i32 + dz) as u16,
            );
            s.push_int(nc.packed() as i32);
        });

        // 1017
        none!(m, PLAYERCOUNT => |s| {
            s.push_int(engine::<E>().playercount() as i32);
        });

        // 1018
        none!(m, PROJANIM_MAP => |s| {
            let arc = s.pop_int_as::<u8>()?;
            let peak = s.pop_int_as::<u8>()?;
            let duration = s.pop_int_as::<u16>()?;
            let delay = s.pop_int_as::<u16>()?;
            let dst_height = s.pop_int_as::<u8>()?;
            let src_height = s.pop_int_as::<u8>()?;
            let spotanim = pop_spotanim(s)?;
            let dst = pop_coord(s)?;
            let src = pop_coord(s)?;
            engine_mut::<E>().map_proj_anim(
                src.y(),
                src.x(),
                src.z(),
                dst.x(),
                dst.z(),
                0,
                spotanim.id,
                src_height << 2,
                dst_height << 2,
                delay,
                duration,
                peak,
                arc
            );
        });

        // 1019
        none!(m, SEQLENGTH => |s| {
            let seq = pop_seq(s)?;
            s.push_int(seq.duration as i32);
        });

        // 1020
        none!(m, SPOTANIM_MAP => |s| {
            let delay = s.pop_int_as::<u16>()?;
            let height = s.pop_int_as::<u8>()?;
            let coord = pop_coord(s)?;
            let spotanim = pop_spotanim(s)?;
            engine_mut::<E>().anim_map(coord.y(), coord.x(), coord.z(), spotanim.id, height, delay);
        });

        // 1021
        none!(m, WORLD_DELAY => |s| {
            // arg is popped elsewhere
            s.execution = ExecutionState::WorldSuspended;
        });

        // 1022
        #[cfg(since_274)]
        none!(m, MIDI_LENGTH => |s| {
            let id = s.pop_int();
            let ticks = midi_tick_length(id)?;
            s.push_int(ticks as i32);
        });

        // 1023
        #[cfg(since_274)]
        none!(m, MAP_LOC => |s| {
            let coord = pop_coord(s)?;
            s.push_int(engine::<E>().map_loc(coord) as i32);
        });

        // 1024
        #[cfg(since_289)]
        none!(m, SOUND_AREA => |s| {
            s.pop_int(); // delay?
            let loops = s.pop_int_as::<u8>()?;
            let synth = s.pop_int();
            let range = s.pop_int_as::<u8>()?;
            let coord = pop_coord(s)?;
            if synth == -1 {
                return Ok(());
            }
            engine_mut::<E>().sound_area(coord.y(), coord.x(), coord.z(), synth as u16, range, loops);
        });
    }
}
