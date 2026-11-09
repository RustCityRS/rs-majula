use crate::engine::{ScriptEngine, engine, engine_mut};
use crate::ops::player::carried_source;
use crate::register::OpsRegistry;
use crate::state::ScriptState;
use crate::util::{pop_coord, pop_script};
use crate::{ControllerUid, ScriptError, handlers, none};
use rs_pack::cache::script::*;

/// The active controller, or with `secondary` the `.active_controller` a
/// dot-prefixed command or var names, as npcs have.
pub(crate) fn active_controller(s: &ScriptState, secondary: bool) -> crate::Result<ControllerUid> {
    if secondary {
        s.active_controller2
            .ok_or_else(|| ScriptError::Runtime("no .active_controller".into()))
    } else {
        s.active_controller
            .ok_or_else(|| ScriptError::Runtime("no active_controller".into()))
    }
}

fn set_active_controller(s: &mut ScriptState, uid: Option<ControllerUid>, secondary: bool) {
    if secondary {
        s.active_controller2 = uid;
    } else {
        s.active_controller = uid;
    }
}

/// Registers the controller opcodes: `CONTROLLER_ADD`, `CONTROLLER_COORD`,
/// `CONTROLLER_DEL`, `CONTROLLER_FINDEXACT`, `CONTROLLER_QUEUE`,
/// `CONTROLLER_QUEUEVARARG` and `CONTROLLER_SETTIMER`.
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 5300
        none!(m, CONTROLLER_COORD => |s| {
            let coord = engine::<E>().controller_coord(active_controller(s, s.int_operand() != 0)?)?;
            s.push_int(coord.packed() as i32);
        });

        // 5301
        none!(m, CONTROLLER_DEL => |s| {
            let secondary = s.int_operand() != 0;
            engine_mut::<E>().controller_del(active_controller(s, secondary)?);
            set_active_controller(s, None, secondary);
        });

        // 5302
        none!(m, CONTROLLER_QUEUE => |s| {
            let delay = s.pop_int_as::<u16>()?;
            let arg = s.pop_int();
            let queue = s.pop_int();
            if queue != 1 {
                Err(ScriptError::Runtime(format!(
                    "controller_queue: a controller has only queue 1, not {queue}"
                )))?;
            }
            engine_mut::<E>().controller_queue(active_controller(s, s.int_operand() != 0)?, arg, delay, carried_source(s))?;
        });

        // 5303
        none!(m, CONTROLLER_FINDEXACT => |s| {
            let id = s.pop_int_as::<u16>()?;
            let coord = pop_coord(s)?;
            let found = engine::<E>().controller_findexact(coord, id);
            if found.is_some() {
                set_active_controller(s, found, s.int_operand() != 0);
            }
            s.push_int(found.is_some() as i32);
        });

        // 5304
        none!(m, CONTROLLER_ADD => |s| {
            let duration = s.pop_int();
            let id = s.pop_int_as::<u16>()?;
            let coord = pop_coord(s)?;
            let uid = engine_mut::<E>().controller_add(coord, id, duration)?;
            set_active_controller(s, Some(uid), s.int_operand() != 0);
        });

        // 5305
        none!(m, CONTROLLER_SETTIMER => |s| {
            let interval = s.pop_int();
            engine_mut::<E>().controller_settimer(active_controller(s, s.int_operand() != 0)?, interval)?;
        });

        // 5306
        none!(m, CONTROLLER_QUEUEVARARG => |s| {
            let args = s.pop_script_args();
            let delay = s.pop_int_as::<u16>()?;
            let script = pop_script::<E>(s)?.id;
            let uid = active_controller(s, s.int_operand() != 0)?;
            engine_mut::<E>().controller_queue_script(uid, script, args, delay, carried_source(s))?;
        });
    }
}
