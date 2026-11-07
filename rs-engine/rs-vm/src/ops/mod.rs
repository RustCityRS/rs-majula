/// Opcode handler modules for the RuneScript virtual machine.
///
/// Each submodule exports a `build()` function that returns an [`OpsRegistry`](crate::register::OpsRegistry)
/// populated with closure-based handlers for a specific category of opcodes. The
/// engine assembles all registries at startup by calling each `build()` and merging
/// them with [`OpsRegistry::extend`](crate::register::OpsRegistry::extend).
///
/// # Modules
///
/// * [`core`] -- stack manipulation, branching, flow control, gosub/goto, switch, locals
/// * [`db`] -- database row/table queries (`DB_FIND`, `DB_GETFIELD`, etc.)
/// * [`debug`] -- development helpers (`CONSOLE`, `ERROR`, `GETTIMESPENT`)
/// * [`r#enum`] -- enum lookup opcodes (`ENUM`, `ENUM_GETOUTPUTCOUNT`)
/// * [`inv`] -- inventory management (add, delete, move, transmit, totals)
/// * [`lc`] -- Loc config lookups (category, name, params, size, etc.)
/// * [`loc`] -- location/scenery opcodes (add, delete, change, find, animate)
/// * [`nc`] -- NPC config lookups (category, name, params, size, etc.)
/// * [`npc`] -- live NPC interaction (movement, combat, AI modes, queues, iterators)
/// * [`number`] -- arithmetic, bitwise, trigonometric, and RNG opcodes
/// * [`obj`] -- ground-item opcodes (add, delete, take, queries)
/// * [`oc`] -- object/item config lookups (cost, wearpos, stackable, params, etc.)
/// * [`player`] -- player interaction (stats, UI, movement, queues, hunting, combat)
/// * [`server`] -- world/map utilities (coord helpers, pathfinding, zone queries, projectiles)
/// * [`string`] -- string manipulation (append, compare, split, substring, etc.)
/// * [`r#struct`] -- struct/param lookups (`STRUCT_PARAM`)
///
/// # Call Stack
///
/// **Called by:** `Engine::new` (in `rs-engine/src/engine.rs`) which calls each
/// submodule's `build()` and merges them via `OpsRegistry::extend`.
pub mod core;
pub mod db;
pub mod debug;
pub mod r#enum;
pub mod inv;
pub mod lc;
pub mod loc;
pub mod nc;
pub mod npc;
pub mod number;
pub mod obj;
pub mod oc;
pub mod player;
pub mod server;
pub mod string;
pub mod r#struct;
/// `#testscript` commands, registered only by the headless runner.
#[cfg(since_647)]
pub mod test;
