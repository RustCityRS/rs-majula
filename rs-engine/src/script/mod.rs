mod engine;
pub(crate) mod errors;
mod npc;
mod player;
mod runner;

use crate::core::Engine;
use rs_vm::ops;
use rs_vm::register::OpsRegistry;

/// Builds and returns the complete [`OpsRegistry`] for the script VM.
///
/// Collects opcode handler functions from the operation modules (core, db, debug,
/// enum, interface, inv, lc, loc, nc, npc, number, obj, oc, player, server, string, struct, vars,
/// region from rev 530 and controller from rev 647) and registers them in a single
/// registry that the VM uses at runtime to dispatch script opcodes.
///
/// # Returns
///
/// A fully populated [`OpsRegistry`] ready to be installed in the engine.
///
/// # Call Stack
///
/// **Called by:** `Engine::new`
/// **Calls:** `ops::core::build`, `ops::db::build`, `ops::debug::build`, etc.
pub fn register_ops() -> OpsRegistry {
    let mut ops = OpsRegistry::new();
    ops.extend(ops::core::build::<Engine>());
    ops.extend(ops::db::build::<Engine>());
    ops.extend(ops::debug::build());
    ops.extend(ops::r#enum::build::<Engine>());
    ops.extend(ops::interface::build());
    ops.extend(ops::inv::build::<Engine>());
    ops.extend(ops::lc::build::<Engine>());
    ops.extend(ops::loc::build::<Engine>());
    ops.extend(ops::nc::build::<Engine>());
    ops.extend(ops::npc::build::<Engine>());
    ops.extend(ops::number::build::<Engine>());
    ops.extend(ops::obj::build::<Engine>());
    ops.extend(ops::oc::build::<Engine>());
    ops.extend(ops::player::build::<Engine>());
    ops.extend(ops::server::build::<Engine>());
    ops.extend(ops::string::build::<Engine>());
    ops.extend(ops::r#struct::build::<Engine>());
    ops.extend(ops::vars::build());
    #[cfg(since_530)]
    ops.extend(ops::region::build::<Engine>());
    #[cfg(since_647)]
    ops.extend(ops::controller::build::<Engine>());
    ops
}

#[cfg(all(test, since_647))]
mod tests {
    use super::register_ops;
    use runec::opcodes::{CONSOLE, IF_SETCOLOUR, IF_SETRESUMEBUTTONS, P_LOGOUT, POP_VARPBIT};

    #[test]
    fn world_registers_shared_opcode_families() {
        let ops = register_ops();
        for opcode in [
            CONSOLE,
            IF_SETCOLOUR,
            IF_SETRESUMEBUTTONS,
            POP_VARPBIT,
            P_LOGOUT,
        ] {
            assert!(ops.contains(opcode), "missing opcode {opcode}");
        }
    }
}
