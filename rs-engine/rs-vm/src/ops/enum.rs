use crate::engine::ScriptEngine;
use crate::register::OpsRegistry;
use crate::util::pop_enum;
use crate::{ScriptError, handlers, none};
use rs_pack::ParamValue;
use rs_pack::cache::provider::CacheType;
use rs_pack::cache::script::{ENUM, ENUM_GETOUTPUTCOUNT};

/// Registers enum lookup opcodes for retrieving values from cache-defined
/// enum tables by key.
///
/// # Opcodes Registered
///
/// - `ENUM` -- looks up a key in an enum and pushes the corresponding
///   int or string value (or the enum's default if the key is absent).
/// - `ENUM_GETOUTPUTCOUNT` -- pushes the total number of entries in an enum.
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::enum::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `none!` macros
pub fn build<E: ScriptEngine + 'static>() -> OpsRegistry {
    handlers! { |m|
        // 4400
        none!(m, ENUM => |s| {
            let key = s.pop_int();
            let e = pop_enum::<E>(s)?;
            let outputtype = s.pop_int();
            let inputtype = s.pop_int();

            if e.inputtype as u8 != inputtype as u8 || e.outputtype as u8 != outputtype as u8 {
                return Err(ScriptError::Runtime(format!(
                    "Enum: {:?} Expected input: {}, Got: {}, Expected output: {}, Got: {}",
                    e.debugname(),
                    e.inputtype as u8,
                    inputtype as u8,
                    e.outputtype as u8,
                    outputtype as u8
                )))
            }

            let Some(value) = e.values.get(&key) else {
                s.push_int(e.default_int); return Ok(());
            };

            match value {
                ParamValue::Int(v) => s.push_int(*v),
                ParamValue::String(v) => s.push_string(v),
            }
        });

        // 4401
        none!(m, ENUM_GETOUTPUTCOUNT => |s| {
            let e = pop_enum::<E>(s)?;
            s.push_int(e.values.len() as i32);
        });
    }
}
