use crate::engine::{ScriptEngine, engine, engine_mut};
use crate::register::OpsRegistry;
use crate::state::ScriptState;
use crate::util::pop_coord;
use crate::{RegionUid, ScriptError, handlers, none};
use runec::opcodes::*;

/// The active region, or with `secondary` the `.active_region` a
/// dot-prefixed command names, as npcs and controllers have.
fn active_region(s: &ScriptState, secondary: bool) -> crate::Result<RegionUid> {
    if secondary {
        s.active_region2
            .ok_or_else(|| ScriptError::Runtime("no .active_region".into()))
    } else {
        s.active_region
            .ok_or_else(|| ScriptError::Runtime("no active_region".into()))
    }
}

fn set_active_region(s: &mut ScriptState, uid: RegionUid, secondary: bool) {
    if secondary {
        s.active_region2 = Some(uid);
    } else {
        s.active_region = Some(uid);
    }
}

/// Registers the region (instance) opcodes: `REGION_FINDBYCOORD`,
/// `REGION_GETCOORD`, `REGION_CREATE`, `REGION_SET`, `REGION_UID`,
/// `REGION_FINDBYUID` and `REGION_SETEXITCOORD`, each with a dot form on
/// the secondary region. A find that misses leaves its region as it was.
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 5200
        none!(m, REGION_FINDBYCOORD => |s| {
            let coord = pop_coord(s)?;
            let found = engine::<E>().region_find(coord);
            if let Some(uid) = found {
                set_active_region(s, uid, s.int_operand() != 0);
            }
            s.push_int(found.is_some() as i32);
        });

        // 5201
        none!(m, REGION_GETCOORD => |s| {
            let z = s.pop_int();
            let x = s.pop_int();
            let level = s.pop_int();
            let uid = active_region(s, s.int_operand() != 0)?;
            let coord = engine::<E>().region_coord(uid, level, x, z)?;
            s.push_int(coord.packed() as i32);
        });

        // 5202
        none!(m, REGION_CREATE => |s| {
            let north = s.pop_int();
            let east = s.pop_int();
            let levels = s.pop_int();
            let uid = engine_mut::<E>().region_create(levels, east, north)?;
            set_active_region(s, uid, s.int_operand() != 0);
            let corner = engine::<E>().region_coord(uid, 0, 0, 0)?;
            s.push_int(corner.packed() as i32);
        });

        // 5203
        none!(m, REGION_SET => |s| {
            let rotation = s.pop_int();
            let source = pop_coord(s)?;
            let north = s.pop_int();
            let east = s.pop_int();
            let level = s.pop_int();
            let uid = active_region(s, s.int_operand() != 0)?;
            engine_mut::<E>().region_set(uid, level, east, north, source, rotation)?;
        });

        // 5204
        none!(m, REGION_UID => |s| {
            let uid = active_region(s, s.int_operand() != 0)?;
            s.push_int(uid.packed() as i32);
        });

        // 5205
        none!(m, REGION_FINDBYUID => |s| {
            let uid = RegionUid::from_packed(s.pop_int() as u32);
            let found = engine::<E>().region_exists(uid);
            if found {
                set_active_region(s, uid, s.int_operand() != 0);
            }
            s.push_int(found as i32);
        });

        // 5206
        none!(m, REGION_SETEXITCOORD => |s| {
            let exit = pop_coord(s)?;
            let uid = active_region(s, s.int_operand() != 0)?;
            engine_mut::<E>().region_set_exit(uid, exit)?;
        });
    }
}
