use crate::register::OpsRegistry;
use crate::{ScriptError, handlers, none};
use rs_pack::cache::script::*;
use std::time::Instant;
use tracing::info;

/// Registers debug and development opcodes for logging messages and
/// tracking script profiling data.
///
/// # Opcodes Registered
///
/// - `CONSOLE` -- pops a string and logs it at info level.
/// - `ERROR` -- pops a string and logs it at error level.
/// - `TIMESPENT` -- records a monotonic timestamp on the script state.
/// - `GETTIMESPENT` -- pushes the time elapsed since `TIMESPENT` (milliseconds,
///   or microseconds when the argument is `1`).
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) via `ops::debug::build`
/// **Calls:** `OpsRegistry::new`, `OpsRegistry::insert` via the `handlers!` / `none!` macros
pub fn build() -> OpsRegistry {
    handlers! { |m|
        // 10000
        none!(m, CONSOLE => |s| {
            let msg = s.pop_string();
            info!("{msg}");
        });

        // 10001
        none!(m, ERROR => |s| {
            let msg = s.pop_string();
            Err(ScriptError::Runtime(msg))?;
        });

        // 10002
        none!(m, GETTIMESPENT => |s| {
            let elapsed = s.timespent.map(|t| t.elapsed()).unwrap_or_default();
            let value = if s.pop_int() == 1 {
                elapsed.as_micros() as i32
            } else {
                elapsed.as_millis() as i32
            };
            s.push_int(value);
        });

        // 10003
        none!(m, TIMESPENT => |s| {
            s.timespent = Some(Instant::now());
        });

        // 10019
        // TODO: this is duplicated with `MAP_LIVE`
        none!(m, MAP_PRODUCTION => |s| {
            #[cfg(debug_assertions)]
            s.push_int(0);
            #[cfg(not(debug_assertions))]
            s.push_int(1);
        });
    }
}
