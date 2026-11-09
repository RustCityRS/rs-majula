#[cfg(since_647)]
use crate::active_player_mut;
#[cfg(since_647)]
use crate::engine::ScriptPlayer;
use crate::engine::{ScriptEngine, engine, engine_mut};
use crate::iterators;
use crate::iterators::LocIteratorState;
#[cfg(since_647)]
use crate::ops::test;
use crate::register::OpsRegistry;
#[cfg(since_647)]
use crate::state::ScriptState;
#[cfg(since_647)]
use crate::util::pop_loc;
use crate::util::{pop_coord, pop_param, pop_seq, set_active_loc};
use crate::{ScriptError, active_loc, handlers, none};
use num_enum::TryFromPrimitive;
use rs_pack::ParamValue;
use rs_pack::cache::script::*;
use rs_pack::types::{LocAngle, LocShape};

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
/// - **Model prefetch (647):** `LOC_PREFETCH`, `LOC_PREFETCHALL`
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::loc::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` /
/// `active_loc!` / `active_player_mut!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 3000
        none!(m, LOC_ADD => |s| {
            let duration = s.pop_int();
            let shape = s.pop_int_as::<u8>()?;
            let angle = s.pop_int_as::<u8>()?;
            let id = s.pop_int_as::<u16>()?;
            let coord = pop_coord(s)?;

            let shape = LocShape::try_from_primitive(shape)
                .map_err(|_| ScriptError::Runtime(format!("Invalid loc shape: {}", shape)))?;
            let angle = LocAngle::try_from_primitive(angle)
                .map_err(|_| ScriptError::Runtime(format!("Invalid loc angle: {}", angle)))?;

            let loc = engine_mut::<E>().add_or_change_loc(coord, id, shape, angle, duration as u64)?;
            set_active_loc(s, loc, s.int_operand() != 0);
        });

        // 3001
        active_loc!(m, LOC_ANGLE => |s, loc| {
            s.push_int(engine::<E>().live_loc(loc).angle as i32);
        });

        // 3002
        // https://x.com/JagexAsh/status/1773801749175812307
        active_loc!(m, LOC_ANIM => |s, loc| {
            let seq = pop_seq::<E>(s)?;
            engine_mut::<E>().anim_loc(loc, seq.id);
        });

        // 3003
        active_loc!(m, LOC_CATEGORY => |s, loc| {
            let loc = engine::<E>().live_loc(loc);
            let category = engine::<E>()
                .locs()
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
            #[cfg(since_647)]
            test::record_call(LOC_CHANGE, "loc_change", &id.to_string());
            if let Some(changed) = engine_mut::<E>().change_loc(loc, id, duration as u64)? {
                set_active_loc(s, changed, s.int_operand() != 0);
            }
        });

        // 3005
        active_loc!(m, LOC_COORD => |s, loc| {
            s.push_int(loc.coord.packed() as i32);
        });

        // 3006
        active_loc!(m, LOC_DEL => |s, loc| {
            engine_mut::<E>().remove_loc(loc, s.pop_int() as u64);
        });

        // 3007
        none!(m, LOC_FIND => |s| {
            let id = s.pop_int();
            let coord = pop_coord(s)?;
            if let Some(loc) = engine::<E>().find_loc(coord, id as u16) {
                set_active_loc(s, loc, s.int_operand() != 0);
                s.push_int(1);
            } else {
                s.push_int(0);
            }
        });

        // 3008
        none!(m, LOC_FINDALLZONE => |s| {
            let locs = iterators::loc_zone::<E>(pop_coord(s)?);
            s.loc_iterator = Some(LocIteratorState { matches: locs, cursor: 0 });
        });

        // 3009
        none!(m, LOC_FINDNEXT => |s| {
            let found = if let Some(iter) = &mut s.loc_iterator
                && iter.cursor < iter.matches.len()
            {
                let loc_ref = iter.matches[iter.cursor];
                iter.cursor += 1;

                set_active_loc(s, loc_ref, s.int_operand() != 0);
                true
            } else {
                false
            };
            s.push_int(found as i32);
        });

        // 3010
        active_loc!(m, LOC_NAME => |s, loc| {
            let loc = engine::<E>().live_loc(loc);
            let loc = engine::<E>()
                .locs()
                .get_by_id(loc.id)
                .ok_or(ScriptError::LocNotFound(loc.id as i32))?;
            s.push_string(loc.name.as_deref().unwrap_or(loc.debugname().unwrap_or("null")));
        });

        // 3011
        active_loc!(m, LOC_PARAM => |s, loc| {
            let param = pop_param::<E>(s)?;
            let loc = engine::<E>().live_loc(loc);
            let value = engine::<E>()
                .locs()
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
            s.push_int(engine::<E>().live_loc(loc).shape as i32);
        });

        // 3013
        active_loc!(m, LOC_TYPE => |s, loc| {
            s.push_int(engine::<E>().live_loc(loc).id as i32);
        });

        // 3014
        #[cfg(since_647)]
        active_player_mut!(m, LOC_PREFETCH => |s, player| {
            let (id, shape) = pop_prefetch::<E>(s)?;
            player.loc_prefetch(id, shape);
        });

        // 3015
        #[cfg(since_647)]
        none!(m, LOC_PREFETCHALL => |s| {
            let (id, shape) = pop_prefetch::<E>(s)?;
            let coord = pop_coord(s)?;
            engine_mut::<E>().loc_prefetchall(coord, id, shape);
        });
    }
}

/// Pops a loc type and shape for `LOC_PREFETCH` / `LOC_PREFETCHALL`.
///
/// Returns the type's id and the shape its models are stored under: the client
/// looks a diagonal centrepiece up as a straight one and every wall decoration
/// as the unoffset straight one, but reads a prefetch's shape as sent.
#[cfg(since_647)]
fn pop_prefetch<E: ScriptEngine + 'static>(s: &mut ScriptState) -> crate::Result<(u16, u8)> {
    let shape = s.pop_int_as::<u8>()?;
    let loc = pop_loc::<E>(s)?;
    let shape = LocShape::try_from_primitive(shape)
        .map_err(|_| ScriptError::Runtime(format!("Invalid loc shape: {}", shape)))?;
    let shape = match shape {
        LocShape::CentrepieceDiagonal => LocShape::CentrepieceStraight,
        LocShape::WallDecorStraightOffset
        | LocShape::WallDecorDiagonalOffset
        | LocShape::WallDecorDiagonalNoOffset
        | LocShape::WallDecorDiagonalBoth => LocShape::WallDecorStraightNoOffset,
        shape => shape,
    };
    Ok((loc.id, shape as u8))
}
