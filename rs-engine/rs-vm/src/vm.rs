use crate::engine::{ScriptEngine, ScriptFault, ScriptPlayer, engine_mut};
use crate::register::OpsRegistry;
use crate::session::{EngineHost, ScriptHost};
use crate::state::{ExecutionState, ScriptState};
use crate::util::get_active_player_mut;
use crate::{Result, ScriptError};
use std::marker::PhantomData;
#[cfg(debug_assertions)]
use std::time::Instant;
use tracing::{error, warn};

const MAX_INSTRUCTIONS: i32 = 500_000;

/// Runs the VM's fetch and dispatch loop on `host` until the script halts,
/// suspends, or fails.
///
/// The loop increments the program counter, fetches the next opcode from
/// the script's bytecode, looks up the handler in the [`OpsRegistry`], and
/// invokes it, until a handler sets `state.execution` to a value other
/// than [`ExecutionState::Running`] (e.g. `Finished`, `Suspended`,
/// `PauseButton`, `CountDialog`, `NpcSuspended`, `WorldSuspended`).
///
/// # Arguments
/// * `state` - The mutable script state containing the program counter,
///   stacks, active entities, and execution status.
/// * `ops` - The opcode handler registry mapping opcode numbers to their
///   handler functions.
/// * `host` - The script resources and the player bound to this execution.
///
/// # Returns
/// `Ok(())` once a handler stops the loop, or the [`ScriptError`] when the
/// instruction count reaches `MAX_INSTRUCTIONS` (500,000), the program
/// counter leaves the script, an opcode has no registered handler, or a
/// handler fails.
///
/// # Call Stack
/// **Called by:** [`execute`], `LobbyPlayer::run_trigger` (rs-lobby/src/script.rs).
/// **Calls:** `OpsRegistry::get`, individual opcode handler functions.
pub fn run(state: &mut ScriptState, ops: &OpsRegistry, host: &mut dyn ScriptHost) -> Result<()> {
    dispatch(state, ops, host).map_err(|(_, e)| e)
}

/// [`run`]'s loop. A failure carries the opcode that raised it, or `None`
/// when it came before an opcode was fetched.
fn dispatch(
    state: &mut ScriptState,
    ops: &OpsRegistry,
    host: &mut dyn ScriptHost,
) -> std::result::Result<(), (Option<u16>, ScriptError)> {
    state.execution = ExecutionState::Running;
    while state.execution == ExecutionState::Running {
        if state.opcount >= MAX_INSTRUCTIONS {
            return Err((
                None,
                ScriptError::Runtime(format!("hit the instruction limit ({MAX_INSTRUCTIONS})")),
            ));
        }
        state.pc += 1;
        state.opcount += 1;
        let opcode = state
            .script
            .opcodes
            .get(state.pc as usize)
            .copied()
            .ok_or_else(|| {
                (
                    None,
                    ScriptError::Runtime(format!("invalid program counter: {}", state.pc)),
                )
            })?;
        let handler = ops
            .get(opcode)
            .ok_or((Some(opcode), ScriptError::UnknownOpcode(opcode)))?;
        handler(state, host).map_err(|e| (Some(opcode), e))?;
    }
    Ok(())
}

/// Runs a world script on the game engine through [`run`], which holds the
/// fetch and dispatch loop.
///
/// When [`run`] fails, the error is logged with the script, program counter
/// and failing opcode, handed to [`report_error`], and the script is
/// `Aborted`.
///
/// In debug builds, scripts that take longer than 1000 microseconds trigger
/// a warning log and an in-game message to active players via [`report`].
///
/// # Arguments
/// * `state` - The mutable script state containing the program counter,
///   stacks, active entities, and execution status.
/// * `ops` - The opcode handler registry mapping opcode numbers to their
///   handler functions.
///
/// # Returns
/// The final [`ExecutionState`] after the script stops.
///
/// # Side Effects
/// * Mutates `state.pc`, `state.opcount`, `state.execution`, and any other
///   state modified by individual opcode handlers.
/// * On error, calls [`report_error`] to log and display the error.
/// * In debug builds, may call [`report`] to display CPU-time warnings.
///
/// # Call Stack
/// **Called by:** `Engine::runescript_vm_execute` (rs-engine/src/script/runner.rs),
/// the game engine's script execution entry point.
/// **Calls:** the loop behind [`run`] with an `EngineHost`,
/// [`report_error`], [`report`] (debug only).
pub fn execute<E: ScriptEngine + 'static>(
    state: &mut ScriptState,
    ops: &OpsRegistry,
) -> ExecutionState {
    #[cfg(debug_assertions)]
    let start = Instant::now();
    if let Err((opcode, e)) = dispatch(state, ops, &mut EngineHost::<E>(PhantomData)) {
        let name = &state.script.info.name;
        match opcode {
            Some(opcode) => error!(
                "Script {name} error at pc={} opcode={opcode}: {e}",
                state.pc
            ),
            None => error!("Script {name} error at pc={}: {e}", state.pc),
        }
        report_error::<E>(state, &e.to_string());
        state.execution = ExecutionState::Aborted;
    }

    #[cfg(debug_assertions)]
    {
        let elapsed = start.elapsed().as_micros();
        if elapsed > 1000 {
            let message = format!(
                "Warning [cpu time]: Script: {}, time: {}us, opcount: {}",
                state.script.info.name, elapsed, state.opcount
            );
            warn!("{}", message);
            report::<E>(state, |player| {
                player.message_game_wrapped(&message);
            });
        }
    }

    state.execution
}

/// Sends a debug report to all active players in the current script state.
///
/// Iterates over both the primary and secondary active player slots and
/// invokes the callback with a mutable `dyn ScriptPlayer` reference for
/// each one that is present and accessible.
///
/// This function is only compiled in debug builds (`#[cfg(debug_assertions)]`).
///
/// # Arguments
/// * `state` - The current script state, used to resolve active player UIDs.
/// * `callback` - A closure that receives a `&mut dyn ScriptPlayer` and can
///   send messages or other updates to that player.
///
/// # Side Effects
/// Whatever the `callback` does to each active player (typically sending
/// in-game messages).
///
/// # Call Stack
/// **Called by:** [`execute`] (CPU-time warnings), [`report_error`]
/// (error display to players).
/// **Calls:** [`get_active_player_mut`](crate::util::get_active_player_mut),
/// the user-supplied `callback`.
#[cfg(debug_assertions)]
fn report<E: ScriptEngine + 'static>(
    state: &mut ScriptState,
    mut callback: impl FnMut(&mut dyn ScriptPlayer),
) {
    for secondary in [false, true] {
        if let Ok(player) = get_active_player_mut::<E>(state, secondary) {
            callback(player);
        }
    }
}

/// Logs a script error with a full stack backtrace, hands it to the engine
/// for staff ([`ScriptEngine::script_error`]) and, in debug builds, displays
/// the same information to active players as in-game messages.
///
/// The backtrace is constructed from the `goto_frame_stack` in `state`,
/// walking backwards from the current frame to the root. Each frame
/// contributes its script name, source file name, and line number.
///
/// # Arguments
/// * `state` - The current script state, providing the active script,
///   program counter, goto-frame stack, and active player references.
/// * `msg` - The error message to log and display.
///
/// # Side Effects
/// * Emits `error!` log lines for the error message, source file, and
///   every frame in the backtrace.
/// * In debug builds, calls [`report`] to send the same information as
///   wrapped game messages to all active players.
///
/// # Call Stack
/// **Called by:** [`execute`] (on unhandled opcodes and handler errors).
/// **Calls:** `tracing::error!`, [`ScriptEngine::script_error`], [`report`]
/// (debug only), [`ScriptPlayer::message_game_wrapped`].
fn report_error<E: ScriptEngine + 'static>(state: &mut ScriptState, msg: &str) {
    let file_name = state.script.info.file_name().to_string();
    let name = state.script.info.name.clone();
    let line = state.script.info.line_number(state.pc);
    let frames: Vec<_> = state
        .goto_frame_stack
        .iter()
        .rev()
        .map(|f| {
            (
                f.script.info.name.clone(),
                f.script.info.file_name().to_string(),
                f.script.info.line_number(f.pc),
            )
        })
        .collect();

    let mut lines = vec![
        format!("script error: {msg}"),
        format!("file: {file_name}"),
        "stack backtrace:".to_string(),
        format!("   1: {name} - {file_name}:{line}"),
    ];
    let mut trace = 1;
    for (name, file_name, line) in &frames {
        trace += 1;
        lines.push(format!("   {trace}: {name} - {file_name}:{line}"));
    }

    for line in &lines {
        error!("{line}");
    }

    engine_mut::<E>().script_error(ScriptFault {
        script: name.into(),
        file: file_name,
        line,
        message: msg.to_owned(),
        trace: frames
            .into_iter()
            .map(|(name, file_name, line)| format!("{name} - {file_name}:{line}"))
            .collect(),
        player: state.active_player.or(state.active_player2),
        npc: state.active_npc.or(state.active_npc2),
    });

    #[cfg(debug_assertions)]
    report::<E>(state, |player| {
        for line in &lines {
            player.message_game_wrapped(line);
        }
    });
}
