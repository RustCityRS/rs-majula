use crate::engine::{ScriptEngine, ScriptPlayer, cache, engine_mut};
use crate::macros::active_obj_ref;
use crate::register::OpsRegistry;
use crate::state::ObjRef;
use crate::util::*;
use crate::{ScriptError, active_obj, handlers, iterators, none};
use rs_grid::CoordGrid;
use rs_pack::ParamValue;
use rs_pack::cache::script::*;

/// Registers ground-item (obj) opcodes for spawning, removing, picking up,
/// and querying items that exist on the world map.
///
/// # Opcodes Registered
///
/// Key opcodes include:
/// - **Spawning:** `OBJ_ADD` (receiver-visible), `OBJ_ADDALL` (globally visible)
/// - **Removal:** `OBJ_DEL`
/// - **Pickup:** `OBJ_TAKEITEM`
/// - **Queries:** `OBJ_COORD`, `OBJ_COUNT`, `OBJ_TYPE`
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::obj::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` /
/// `active_obj!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 3500
        // https://x.com/JagexAsh/status/1679942100249464833
        // https://x.com/NobodyImpo74600/status/1791469645939065036
        none!(m, OBJ_ADD => |s| {
            let duration = s.pop_int();
            let count = s.pop_int();
            let obj_id = s.pop_int();
            let coord = s.pop_int() as u32;

            if obj_id == -1 || count == -1 {
                return Ok(());
            }

            let obj_type = cache()
                .objs
                .get_by_id(obj_id as u16)
                .ok_or(ScriptError::ObjNotFound(obj_id))?;

            if obj_type.dummyitem as u8 != 0 {
                return Err(ScriptError::Runtime(format!(
                    "attempted to add dummy item: {}",
                    obj_id
                )));
            }

            let id = obj_id as u16;
            let count = count as u32;
            let receiver37 = s.active_player.map(|uid| uid.username37());

            add_obj_split::<E>(coord, id, count, obj_type.stackable, receiver37, duration as u64);

            set_active_obj(s, ObjRef { coord, id, count }, s.int_operand() != 0);
        });

        // 3501
        // https://x.com/JagexAsh/status/1778879334167548366
        none!(m, OBJ_ADDALL => |s| {
            let duration = s.pop_int();
            let count = s.pop_int();
            let obj_id = s.pop_int();
            let coord = s.pop_int() as u32;

            if obj_id == -1 || count == -1 {
                return Ok(());
            }

            let obj_type = cache()
                .objs
                .get_by_id(obj_id as u16)
                .ok_or(ScriptError::ObjNotFound(obj_id))?;

            let id = obj_id as u16;
            let count = count as u32;

            add_obj_split::<E>(coord, id, count, obj_type.stackable, None, duration as u64);

            set_active_obj(s, ObjRef { coord, id, count }, s.int_operand() != 0);
        });

        // 3502
        active_obj!(m, OBJ_COORD => |s, obj| {
            s.push_int(obj.coord as i32);
        });

        // 3503
        active_obj!(m, OBJ_COUNT => |s, obj| {
            s.push_int(if obj.count > 0 { obj.count as i32 } else { 0 });
        });

        // 3504
        active_obj!(m, OBJ_DEL => |s, obj| {
            let respawnrate = cache()
                .objs
                .get_by_id(obj.id)
                .map(|t| t.respawnrate as u64)
                .unwrap_or(100);
            let user37 = s.active_player.map(|uid| uid.username37());
            engine_mut::<E>().remove_obj(obj.coord, obj.id, user37, respawnrate);
        });

        // 3505
        none!(m, OBJ_FIND => |s| {
            let id = s.pop_int_as::<u16>()?;
            let coord = s.pop_int() as u32;
            let receiver37 = s.active_player.map(|uid| uid.username37());

            if let Some(obj) = engine_mut::<E>().find_obj(coord, id, receiver37) {
                set_active_obj(s, obj, s.int_operand() != 0);
                s.push_int(1);
            } else {
                s.push_int(0);
            }
        });

        // 3506
        none!(m, OBJ_FINDALLZONE => |s| {
            let coord = CoordGrid::from(s.pop_int() as u32);
            let objs = iterators::obj_zone::<E>(coord);
            s.obj_iterator = Some(iterators::ObjIteratorState {
                matches: objs,
                cursor: 0,
            });
        });

        // 3507
        none!(m, OBJ_FINDNEXT => |s| {
            let iter = match s.obj_iterator.as_mut() {
                Some(iter) => iter,
                None => {
                    s.push_int(0);
                    return Ok(());
                }
            };
            if iter.cursor < iter.matches.len() {
                let obj = iter.matches[iter.cursor];
                iter.cursor += 1;
                set_active_obj(s, obj, s.int_operand() != 0);
                s.push_int(1);
            } else {
                s.push_int(0);
            }
        });

        // 3508
        active_obj!(m, OBJ_NAME => |s, obj| {
            let obj_type = cache()
                .objs
                .get_by_id(obj.id)
                .ok_or(ScriptError::ObjNotFound(obj.id as i32))?;
            s.push_string(obj_type.name.as_deref().unwrap_or(obj_type.debugname().unwrap_or("null")));
        });

        // 3509
        active_obj!(m, OBJ_PARAM => |s, obj| {
            let param = pop_param(s)?;
            let value = cache()
                .objs
                .get_by_id(obj.id)
                .ok_or(ScriptError::ParamNotFound(param.id as i32))?
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

        // 3510
        // https://x.com/JagexAsh/status/1679942100249464833
        none!(m, OBJ_TAKEITEM => |s| {
            require_active_obj(s)?;
            let inv_type = pop_inv(s)?;

            let obj = active_obj_ref(s)?;
            if obj.count == 0 {
                return Ok(());
            }

            let obj_type = cache()
                .objs
                .get_by_id(obj.id)
                .ok_or(ScriptError::ObjNotFound(obj.id as i32))?;

            let uid = s
                .active_player
                .ok_or_else(|| ScriptError::Runtime("no active_player".into()))?;

            let coord = CoordGrid::from(obj.coord);

            let stackable = obj_type.stackable;
            let obj_id = obj.id;

            let player = engine_mut::<E>()
                .get_player_mut(uid.pid())
                .ok_or_else(|| {
                    ScriptError::Runtime(format!("active player slot empty: {}", uid.pid()))
                })?;

            let user37 = player.uid().username37();
            let overflow = get_inv_mut::<E>(inv_type.id, player)?.add(obj_id, obj.count, stackable);
            if overflow > 0 {
                let player_coord = player.coord();
                add_obj_split::<E>(player_coord, obj_id, overflow, stackable, Some(user37), LOOTDROP_DURATION);
            }

            engine_mut::<E>().remove_obj(coord.packed(), obj_id, Some(user37), obj_type.respawnrate as u64);
        });

        // 3511
        active_obj!(m, OBJ_TYPE => |s, obj| {
            s.push_int(obj.id as i32);
        });

        // obj_setvar // https://x.com/JagexAsh/status/1679942100249464833
        // obj_adddelayed // https://x.com/JagexAsh/status/1730321158858276938
    }
}
