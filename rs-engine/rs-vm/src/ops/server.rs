#[cfg(since_274)]
use crate::ScriptError;
use crate::engine::{ScriptEngine, cache, engine, engine_mut};
use crate::register::OpsRegistry;
use crate::state::ExecutionState;
use crate::util::{pop_seq, pop_spotanim};
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
            let coord = CoordGrid::from(s.pop_int() as u32);
            s.push_int(coord.x() as i32);
        });

        // 1001
        none!(m, COORDY => |s| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            s.push_int(coord.y() as i32);
        });

        // 1002
        none!(m, COORDZ => |s| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            s.push_int(coord.z() as i32);
        });

        // 1003
        none!(m, DISTANCE => |s| {
            let b = CoordGrid::from(s.pop_int() as u32);
            let a = CoordGrid::from(s.pop_int() as u32);
            s.push_int(a.distance(b));
        });

        // 1004
        none!(m, INZONE => |s| {
            let test = CoordGrid::from(s.pop_int() as u32);
            let ne = CoordGrid::from(s.pop_int() as u32);
            let sw = CoordGrid::from(s.pop_int() as u32);
            let ok = test.y() == sw.y()
                && test.x() >= sw.x()
                && test.x() <= ne.x()
                && test.z() >= sw.z()
                && test.z() <= ne.z();
            s.push_int(ok as i32);
        });

        // 1005
        none!(m, LINEOFSIGHT => |s| {
            let dst = CoordGrid::from(s.pop_int() as u32);
            let src = CoordGrid::from(s.pop_int() as u32);
            if src.y() != dst.y() {
                s.push_int(0);
                return Ok(());
            }
            let engine = engine::<E>();
            if !engine.members() && !cache().is_free(dst.x(), dst.z()) {
                s.push_int(0);
                return Ok(());
            }
            s.push_int(engine.lineofsight(src, dst) as i32);
        });

        // 1006
        none!(m, LINEOFWALK => |s| {
            let dst = CoordGrid::from(s.pop_int() as u32);
            let src = CoordGrid::from(s.pop_int() as u32);
            if src.y() != dst.y() {
                s.push_int(0);
                return Ok(());
            }
            let engine = engine::<E>();
            if !engine.members() && !cache().is_free(dst.x(), dst.z()) {
                s.push_int(0);
                return Ok(());
            }
            s.push_int(engine.lineofwalk(src, dst) as i32);
        });

        // 1007
        none!(m, MAP_BLOCKED => |s| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            let engine = engine::<E>();
            if !engine.members() && !cache().is_free(coord.x(), coord.z()) {
                s.push_int(1);
                return Ok(());
            }
            s.push_int(engine.map_blocked(coord) as i32);
        });

        // 1008
        none!(m, MAP_CLOCK => |s| {
            s.push_int(engine::<E>().clock() as i32);
        });

        // 1009
        none!(m, MAP_FINDSQUARE => |s| {
            let find_type = s.pop_int();
            let max_radius = s.pop_int();
            let min_radius = s.pop_int();
            let coord = CoordGrid::from(s.pop_int() as u32);

            let engine = engine::<E>();
            let engine_mut = engine_mut::<E>();
            let cache = cache();
            let free_world = !engine.members();

            if max_radius < 10 {
                for _ in 0..50 {
                    let dx = (engine_mut.random().next_double() * (2 * max_radius + 1) as f64) as i32 - max_radius;
                    let dz = (engine_mut.random().next_double() * (2 * max_radius + 1) as f64) as i32 - max_radius;
                    let distance = dx.abs().max(dz.abs());
                    if distance < min_radius || distance > max_radius {
                        continue;
                    }
                    let rx = (coord.x() as i32 + dx) as u16;
                    let rz = (coord.z() as i32 + dz) as u16;
                    if free_world && !cache.is_free(rx, rz) {
                        continue;
                    }
                    let src = CoordGrid::new(rx, coord.y(), rz);
                    let blocked = engine.map_blocked(src);
                    let vis_ok = match find_type {
                        1 => engine.lineofwalk(src, coord),
                        2 => engine.lineofsight(src, coord),
                        _ => true,
                    };
                    if vis_ok && !blocked {
                        s.push_int(src.packed() as i32);
                        return Ok(());
                    }
                }
            } else {
                for x in (coord.x() as i32 - max_radius)..=(coord.x() as i32 + max_radius) {
                    let dx = x - coord.x() as i32;
                    let dz = (engine_mut.random().next_double() * (2 * max_radius + 1) as f64) as i32 - max_radius;
                    let distance = dx.abs().max(dz.abs());
                    if distance < min_radius || distance > max_radius {
                        continue;
                    }
                    let rx = x as u16;
                    let rz = (coord.z() as i32 + dz) as u16;
                    if free_world && !cache.is_free(rx, rz) {
                        continue;
                    }
                    let src = CoordGrid::new(rx, coord.y(), rz);
                    let blocked = engine.map_blocked(src);
                    let too_close = src.in_distance(coord, min_radius as u8);
                    let vis_ok = match find_type {
                        1 => engine.lineofwalk(src, coord),
                        2 => engine.lineofsight(src, coord),
                        _ => true,
                    };
                    if vis_ok && !blocked && !too_close {
                        s.push_int(src.packed() as i32);
                        return Ok(());
                    }
                }
            }
            s.push_int(coord.packed() as i32);
        });

        // 1010
        none!(m, MAP_INDOORS => |s| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            s.push_int(engine::<E>().map_indoors(coord) as i32);
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
            let coord = CoordGrid::from(s.pop_int() as u32);
            s.push_int(engine::<E>().locaddunsafe(coord) as i32);
        });

        // 1013
        none!(m, MAP_MEMBERS => |s| {
            s.push_int(engine::<E>().members() as i32);
        });

        // 1014
        none!(m, MAP_MULTIWAY => |s| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            s.push_int(cache().is_multi(coord.x(), coord.z(), coord.y()) as i32);
        });

        // 1015
        none!(m, MAP_PLAYERCOUNT => |s| {
            let to = CoordGrid::from(s.pop_int() as u32);
            let from = CoordGrid::from(s.pop_int() as u32);

            let mut count = 0;
            let from_zx = from.x() >> 3;
            let from_zz = from.z() >> 3;
            let to_zx = (to.x() + 7) >> 3;
            let to_zz = (to.z() + 7) >> 3;

            let engine = engine::<E>();

            for zx in from_zx..=to_zx {
                for zz in from_zz..=to_zz {
                    let coords = engine.get_zone_player_coords(zx << 3, from.y(), zz << 3);
                    for packed in coords {
                        let coord = CoordGrid::from(packed);
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
            let base = CoordGrid::from(s.pop_int() as u32);
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
            let dst = CoordGrid::from(s.pop_int() as u32);
            let src = CoordGrid::from(s.pop_int() as u32);
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
            let coord = CoordGrid::from(s.pop_int() as u32);
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
            let track = s.pop_int();
            let ticks = cache()
                .midi_tick_lengths
                .get(track as usize)
                .copied()
                .flatten()
                .ok_or(ScriptError::SongNotFound(track))?;
            s.push_int(ticks as i32);
        });

        // 1023
        #[cfg(since_274)]
        none!(m, MAP_LOC => |s| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            s.push_int(engine::<E>().map_loc(coord) as i32);
        });
    }
}
