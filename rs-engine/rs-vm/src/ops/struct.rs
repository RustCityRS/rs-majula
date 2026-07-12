use crate::engine::ScriptEngine;
use crate::register::OpsRegistry;
use crate::util::{pop_param, pop_struct};
use crate::{handlers, none};
use rs_pack::ParamValue;
use rs_pack::cache::script::STRUCT_PARAM;

/// Registers the struct/param lookup opcode for reading typed parameters
/// from cache-defined struct definitions.
///
/// # Opcodes Registered
///
/// - `STRUCT_PARAM` -- given a struct id and a param id, pushes the
///   param's int or string value (falling back to the param's default).
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::struct::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 4700
        none!(m, STRUCT_PARAM => |s| {
            let param = pop_param::<E>(s)?;
            let id = pop_struct::<E>(s)?;
            let value = id.params
                .as_ref()
                .and_then(|p| param.get_param_or_default(p))
                .cloned()
                .unwrap_or_else(|| param.default_param());
            match value {
                ParamValue::Int(v) => s.push_int(v),
                ParamValue::String(v) => s.push_string(&v),
            }
        });
    }
}
