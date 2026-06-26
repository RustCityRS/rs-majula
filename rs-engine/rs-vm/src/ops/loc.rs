use crate::engine::{ScriptEngine, cache, engine, engine_mut};
use crate::iterators;
use crate::iterators::LocIteratorState;
use crate::register::OpsRegistry;
use crate::state::LocRef;
use crate::util::{pop_param, pop_seq, set_active_loc};
use crate::{ScriptError, active_loc, handlers, none};
use rs_grid::CoordGrid;
use rs_pack::ParamValue;
use rs_pack::cache::script::*;
use rs_pack::types::LocShape;

/// Registers location (scenery) opcodes for adding, removing, changing,
/// finding, animating, and querying world locations.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Lifecycle:** `LOC_ADD`, `LOC_DEL`, `LOC_CHANGE`
/// - **Queries:** `LOC_COORD`, `LOC_TYPE`, `LOC_ANGLE`, `LOC_SHAPE`,
///   `LOC_CATEGORY`, `LOC_PARAM`
/// - **Animation:** `LOC_ANIM`
/// - **Search / iterators:** `LOC_FIND`, `LOC_FINDALLZONE`, `LOC_FINDNEXT`
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::loc::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` /
/// `active_loc!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 3000
        none!(m, LOC_ADD => |s| {
            let duration = s.pop_int();
            let shape = s.pop_int_as::<u8>()?;
            let angle = s.pop_int_as::<u8>()?;
            let id = s.pop_int_as::<u16>()?;
            let coord = s.pop_int() as u32;

            let layer = LocShape::try_from(shape)
                .map_err(|_| ScriptError::Runtime(format!("invalid loc shape: {}", shape)))?
                .layer() as u8;

            engine_mut::<E>().add_or_change_loc(coord, id, shape, angle, duration as u64, true);

            let secondary = s.int_operand() != 0;
            set_active_loc(s, LocRef { coord, id, shape, angle, layer }, secondary);
        });

        // 3001
        active_loc!(m, LOC_ANGLE => |s, loc| {
            s.push_int(loc.angle as i32);
        });

        // 3002
        // https://x.com/JagexAsh/status/1773801749175812307
        active_loc!(m, LOC_ANIM => |s, loc| {
            let seq = pop_seq(s)?;
            engine_mut::<E>().anim_loc(loc.coord, loc.id, seq.id);
        });

        // 3003
        active_loc!(m, LOC_CATEGORY => |s, loc| {
            let category = cache()
                .locs
                .get_by_id(loc.id)
                .ok_or(ScriptError::LocNotFound(loc.id as i32))?
                .category
                .map(|c| c as i32)
                .unwrap_or(-1);
            s.push_int(category);
        });

        // 3004
        active_loc!(m, LOC_CHANGE => |s, loc| {
            let duration = s.pop_int();
            let id = s.pop_int_as::<u16>()?;
            engine_mut::<E>().add_or_change_loc(loc.coord, id, loc.shape, loc.angle, duration as u64, false);
        });

        // 3005
        active_loc!(m, LOC_COORD => |s, loc| {
            s.push_int(loc.coord as i32);
        });

        // 3006
        active_loc!(m, LOC_DEL => |s, loc| {
            let duration = s.pop_int();
            engine_mut::<E>().remove_loc(loc.coord, loc.layer, duration as u64);
        });

        // 3007
        none!(m, LOC_FIND => |s| {
            let id = s.pop_int();
            let coord = CoordGrid::from(s.pop_int() as u32);
            if let Some(loc) = engine::<E>().find_loc(coord.x(), coord.z(), coord.y(), id as u16) {
                set_active_loc(s, loc, s.int_operand() != 0);
                s.push_int(1);
            } else {
                s.push_int(0);
            }
        });

        // 3008
        none!(m, LOC_FINDALLZONE => |s| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            let locs = iterators::loc_zone::<E>(coord);
            s.loc_iterator = Some(LocIteratorState { matches: locs, cursor: 0 });
        });

        // 3009
        none!(m, LOC_FINDNEXT => |s| {
            let found = if let Some(iter) = &mut s.loc_iterator
                && iter.cursor < iter.matches.len()
            {
                let loc_ref = iter.matches[iter.cursor];
                iter.cursor += 1;

                let secondary = s.int_operand() != 0;
                set_active_loc(s, loc_ref, secondary);
                true
            } else {
                false
            };
            s.push_int(if found { 1 } else { 0 });
        });

        // 3010
        active_loc!(m, LOC_NAME => |s, loc| {
            let loc = cache()
                .locs
                .get_by_id(loc.id)
                .ok_or(ScriptError::LocNotFound(loc.id as i32))?;
            s.push_string(loc.name.as_deref().unwrap_or(loc.debugname().unwrap_or("null")));
        });

        // 3011
        active_loc!(m, LOC_PARAM => |s, loc| {
            let param = pop_param(s)?;
            let value = cache()
                .locs
                .get_by_id(loc.id)
                .ok_or(ScriptError::LocNotFound(loc.id as i32))?
                .params
                .as_ref()
                .and_then(|p| param.get_param_or_default(p))
                .cloned()
                .unwrap_or_else(|| param.default_param());
            match value {
                ParamValue::Int(v) => s.push_int(v),
                ParamValue::String(v) => s.push_string(&v),
            }
        });

        // 3012
        active_loc!(m, LOC_SHAPE => |s, loc| {
            s.push_int(loc.shape as i32);
        });

        // 3013
        active_loc!(m, LOC_TYPE => |s, loc| {
            let loc = cache()
                .locs
                .get_by_id(loc.id)
                .ok_or(ScriptError::LocNotFound(loc.id as i32))?;
            s.push_int(loc.id as i32);
        });
    }
}
