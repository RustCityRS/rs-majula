use crate::engine::{ScriptEngine, engine, engine_mut};
use crate::register::OpsRegistry;
use crate::state::ScriptState;
use crate::util::pop_coord;
use crate::{ScriptError, handlers, none};
use rs_pack::cache::script::*;

fn active_region(s: &ScriptState) -> crate::Result<u32> {
    s.active_region
        .ok_or_else(|| ScriptError::Runtime("no active_region".into()))
}

/// Registers the region (instance) opcodes: `REGION_FINDBYCOORD`,
/// `REGION_GETCOORD`, `REGION_CREATE`, `REGION_DEL`, `REGION_COPYZONE`,
/// `REGION_COPYAREA`, `REGION_UID` and `REGION_FINDUID`.
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 5200
        none!(m, REGION_FINDBYCOORD => |s| {
            let coord = pop_coord(s)?;
            s.active_region = engine::<E>().region_find(coord);
            s.push_int(s.active_region.is_some() as i32);
        });

        // 5205
        none!(m, REGION_GETCOORD => |s| {
            let z = s.pop_int();
            let y = s.pop_int();
            let x = s.pop_int();
            let coord = engine::<E>().region_coord(active_region(s)?, x, y, z)?;
            s.push_int(coord.packed() as i32);
        });

        // 5206
        none!(m, REGION_CREATE => |s| {
            let size = s.pop_int();
            s.active_region = engine_mut::<E>().region_create(size)?;
            s.push_int(s.active_region.is_some() as i32);
        });

        // 5207
        none!(m, REGION_DEL => |s| {
            engine_mut::<E>().region_del(active_region(s)?)?;
            s.active_region = None;
        });

        // 5208
        none!(m, REGION_COPYZONE => |s| {
            let rotation = s.pop_int();
            let to = pop_coord(s)?;
            let from = pop_coord(s)?;
            engine_mut::<E>().region_copy(active_region(s)?, from, from, to, rotation)?;
        });

        // 5209
        none!(m, REGION_COPYAREA => |s| {
            let rotation = s.pop_int();
            let to_sw = pop_coord(s)?;
            let from_ne = pop_coord(s)?;
            let from_sw = pop_coord(s)?;
            engine_mut::<E>().region_copy(active_region(s)?, from_sw, from_ne, to_sw, rotation)?;
        });

        // 5210
        none!(m, REGION_UID => |s| {
            let uid = active_region(s)?;
            s.push_int(uid as i32);
        });

        // 5211
        none!(m, REGION_FINDUID => |s| {
            let uid = s.pop_int() as u32;
            s.active_region = engine::<E>().region_exists(uid).then_some(uid);
            s.push_int(s.active_region.is_some() as i32);
        });
    }
}
