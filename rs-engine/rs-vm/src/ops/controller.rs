use crate::engine::{ScriptEngine, engine, engine_mut};
use crate::register::OpsRegistry;
use crate::state::ScriptState;
use crate::util::pop_coord;
use crate::{ControllerUid, ScriptError, handlers, none};
use rs_pack::cache::script::*;

pub(crate) fn active_controller(s: &ScriptState) -> crate::Result<ControllerUid> {
    s.active_controller
        .ok_or_else(|| ScriptError::Runtime("no active_controller".into()))
}

/// Registers the controller opcodes: `CONTROLLER_ADD`, `CONTROLLER_COORD`,
/// `CONTROLLER_DEL`, `CONTROLLER_FINDEXACT`, `CONTROLLER_QUEUE` and
/// `CONTROLLER_SETTIMER`.
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 5201
        none!(m, CONTROLLER_COORD => |s| {
            let coord = engine::<E>().controller_coord(active_controller(s)?)?;
            s.push_int(coord.packed() as i32);
        });

        // 5202
        none!(m, CONTROLLER_DEL => |s| {
            engine_mut::<E>().controller_del(active_controller(s)?);
            s.active_controller = None;
        });

        // 5203
        none!(m, CONTROLLER_QUEUE => |s| {
            let delay = s.pop_int();
            let arg = s.pop_int();
            let queue = s.pop_int();
            if queue != 1 {
                Err(ScriptError::Runtime(format!(
                    "controller_queue: a controller has only queue 1, not {queue}"
                )))?;
            }
            engine_mut::<E>().controller_queue(active_controller(s)?, arg, delay)?;
        });

        // 5204
        none!(m, CONTROLLER_FINDEXACT => |s| {
            let id = s.pop_int_as::<u16>()?;
            let coord = pop_coord(s)?;
            s.active_controller = engine::<E>().controller_findexact(coord, id);
            s.push_int(s.active_controller.is_some() as i32);
        });

        // 5212
        none!(m, CONTROLLER_ADD => |s| {
            let duration = s.pop_int();
            let id = s.pop_int_as::<u16>()?;
            let coord = pop_coord(s)?;
            s.active_controller = Some(engine_mut::<E>().controller_add(coord, id, duration)?);
        });

        // 5213
        none!(m, CONTROLLER_SETTIMER => |s| {
            let interval = s.pop_int();
            engine_mut::<E>().controller_settimer(active_controller(s)?, interval)?;
        });
    }
}
