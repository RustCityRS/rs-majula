use crate::active_player::EnginePlayer;
use crate::controls::DISABLED_MESSAGE;
use crate::core::Engine;
use rs_entity::{MODAL_MAIN, MODAL_NONE, NpcUid, PlayerUid};
use rs_pack::cache::script::Script;
use rs_vm::engine::with_engine;
use rs_vm::pointer::ScriptPointer;
use rs_vm::register::OpsRegistry;
use rs_vm::state::*;
use rs_vm::subject::ScriptSubject;
use rs_vm::trigger::ServerTriggerType;
use rs_vm::{ScriptError, vm};
use std::sync::Arc;
use tracing::error;

impl Engine {
    /// The script a queue entry runs: its `[npc_queue]` or
    /// `[controller_queue]` script, or else the subject type's script for
    /// the trigger it holds.
    pub(crate) fn queued_script(
        &self,
        request: &QueuedScript,
        type_id: u16,
        category: Option<i32>,
    ) -> Option<Arc<Script>> {
        if request.named {
            return self.scripts.get_by_id(request.script_id).cloned();
        }
        let trigger = ServerTriggerType::try_from(request.script_id as u8).ok()?;
        self.script_by_key(trigger, Some(type_id as i32), category)
    }

    /// Finds the script bound to a server trigger for a type id or category.
    ///
    /// Each lookup key holds the trigger ordinal in its low 8 bits, a tag in
    /// bits 8-9 (2 for a type id, 1 for a category) and the type id or
    /// category from bit 10 up. The method tries the most specific key first
    /// (type id), then category, and finally falls back to the bare trigger
    /// ordinal.
    ///
    /// # Arguments
    ///
    /// * `trigger` -- The server trigger type (e.g. `AiSpawn`, `OpObj1`).
    /// * `t` -- Optional type id to specialise the lookup (e.g. an npc or obj
    ///   type, a stat, or a component).
    /// * `c` -- Optional category id for category-level script matching; `-1`
    ///   counts as none.
    ///
    /// # Returns
    ///
    /// The bound [`Script`] for the most specific matching key (type id, then
    /// category, then the bare trigger ordinal), or `None` if no script is
    /// bound.
    pub fn script_by_key(
        &self,
        trigger: ServerTriggerType,
        t: Option<i32>,
        c: Option<i32>,
    ) -> Option<Arc<Script>> {
        let base = trigger as i32;

        if let Some(t) = t {
            // Widen through i64 before truncating: rev-647 interface triggers
            // key by the 32-bit component hash, and the compiler computes the
            // same wrapped key. For sub-2^21 subjects (all 2004-era ids) this
            // is bit-identical to the old u16 shift.
            let key = (((t as i64) << 10) | (0x2 << 8) | base as i64) as i32;
            if let Some(script) = self.scripts.get_by_lookup(key) {
                return Some(Arc::clone(script));
            }
        }

        if let Some(c) = c
            && c != -1
        {
            let key = base | (0x1 << 8) | (c << 10);
            if let Some(script) = self.scripts.get_by_lookup(key) {
                return Some(Arc::clone(script));
            }
        }

        self.scripts.get_by_lookup(base).cloned()
    }

    /// Whether `trigger` has a script for type `t` itself, or (with no `t`)
    /// for category `c` itself: not counting what [`script_by_key`] would
    /// fall back to.
    ///
    /// [`script_by_key`]: Self::script_by_key
    pub fn has_script_for(&self, trigger: ServerTriggerType, t: Option<i32>, c: i32) -> bool {
        let base = trigger as i32;
        let key = match t {
            Some(t) => base | (0x2 << 8) | (t << 10),
            None if c != -1 => base | (0x1 << 8) | (c << 10),
            None => return false,
        };
        self.scripts.get_by_lookup(key).is_some()
    }

    /// Executes a single RuneScript VM invocation against the given script state.
    ///
    /// Temporarily installs `self` as the global engine (via `with_engine`) so
    /// that VM opcodes can access world state through the `ScriptEngine` trait,
    /// then runs the VM until the script finishes, suspends, or aborts.
    ///
    /// # Arguments
    ///
    /// * `state` -- The mutable script state (program counter, stack, locals, etc.).
    ///
    /// # Returns
    ///
    /// The [`ExecutionState`] indicating how the script exited: `Finished`,
    /// `Aborted`, `Suspended` (player), `WorldSuspended`, or `NpcSuspended`.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `runescript_execute_script_player`, `runescript_execute_script_npc`,
    /// `run_world_script`, `logouts`.
    /// **Calls:** `vm::execute`.
    pub fn runescript_vm_execute(&mut self, state: &mut ScriptState) -> ExecutionState {
        self.flush_input_paths();
        let ops = &self.ops as *const OpsRegistry;
        with_engine(self, move || vm::execute::<Engine>(state, unsafe { &*ops }))
    }

    /// Executes a pre-built [`ScriptState`] against the appropriate subject entity.
    ///
    /// Routes execution to `runescript_execute_script_player` or
    /// `runescript_execute_script_npc` depending on the subject variant.
    /// `Loc` and `Obj` subjects run nothing.
    ///
    /// Unless the script suspends (its state then waits on the player, the
    /// NPC or the world queue), the state goes back into `reusable_script`
    /// for the next [`build_state`](Self::build_state) or `run_script_inner`
    /// to reuse, as does a fresh player script that a kill switch keeps from
    /// starting.
    ///
    /// # Arguments
    ///
    /// * `state` -- The fully initialized script state to execute.
    /// * `subject` -- The entity that owns this script execution. `None`
    ///   returns `ScriptError::NoSubject`.
    /// * `protect` -- If `Some(true)`, marks the player as protected for the
    ///   duration of the script (prevents interruption by other scripts).
    /// * `force` -- If `Some(true)`, runs even if the player is already
    ///   protected or delayed.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or `Err(ScriptError::NoSubject)` if no subject
    /// was provided.
    ///
    /// # Call Stack
    ///
    /// **Called by:** the player and NPC phases (resumed scripts, events,
    /// timers, queues), input handlers that start or resume scripts,
    /// `process_walktrigger`, `apply_display_name_result`.
    /// **Calls:** `runescript_execute_script_player`, `runescript_execute_script_npc`.
    pub fn run_script_by_state(
        &mut self,
        state: ScriptState,
        subject: Option<ScriptSubject>,
        protect: Option<bool>,
        force: Option<bool>,
    ) -> Result<(), ScriptError> {
        let Some(subject) = subject else {
            return Err(ScriptError::NoSubject);
        };
        // Only a fresh start: a suspended script resuming finishes, even
        // under a kill switch put on since it began.
        if let ScriptSubject::Player(uid) = subject
            && state.execution == ExecutionState::Running
            && self.switched_off(uid, &state.script)
        {
            self.reusable_script = Some(state);
            return Ok(());
        }
        let returned = match subject {
            ScriptSubject::Player(uid) => {
                let protect = protect.unwrap_or(false);
                let force = force.unwrap_or(false);
                self.runescript_execute_script_player(uid, state, protect, force)
            }
            ScriptSubject::Npc(uid) => self.runescript_execute_script_npc(uid, state),
            ScriptSubject::Loc(_) | ScriptSubject::Obj(_) => Some(state),
        };
        if let Some(returned_state) = returned {
            self.reusable_script = Some(returned_state);
        }
        Ok(())
    }

    /// Whether a kill switch keeps player `uid` from starting `script`; if so,
    /// they're told it's switched off.
    #[inline]
    fn switched_off(&mut self, uid: PlayerUid, script: &Script) -> bool {
        if !self.controls.is_disabled(script.id) {
            return false;
        }
        if let Some(active) = self.get_player_mut(uid.pid()) {
            active.message_game(DISABLED_MESSAGE);
        }
        true
    }

    /// Builds a [`ScriptState`], reusing a pooled state's heap buffers when one
    /// is available (falling back to [`ScriptState::init`]). Mirrors the pool
    /// logic in [`run_script_inner`](Self::run_script_inner); pair it with the
    /// reclaim in [`run_script_by_state`](Self::run_script_by_state) so per-tick
    /// timer/queue scripts cycle a single state instead of allocating ~4 KB
    /// each. `last_int` (if needed) must be set by the caller after building, as
    /// `reset` nulls it.
    pub fn build_state(
        &mut self,
        script: Arc<Script>,
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
        args: Option<Vec<ScriptArgument>>,
    ) -> ScriptState {
        if let Some(mut reusable) = self.reusable_script.take() {
            reusable.reset(script, subject, target, args);
            reusable
        } else {
            ScriptState::init(script, subject, target, args)
        }
    }

    /// Looks up a script by server trigger and executes it against a subject entity.
    ///
    /// Resolves the trigger triple `(type, type_id, category)` to its most
    /// specific bound script via [`Engine::script_by_key`] and hands it to
    /// [`Engine::run_script_inner`], which builds the [`ScriptState`] and
    /// runs it.
    ///
    /// # Arguments
    ///
    /// * `trigger` -- Tuple of `(ServerTriggerType, Option<type_id>, Option<category>)`.
    /// * `subject` -- The entity that owns the script execution.
    /// * `target` -- Optional secondary entity (e.g. interaction target).
    /// * `protect` -- Whether the owning player should be marked as protected.
    /// * `force` -- Whether to bypass protection/delay checks.
    /// * `args` -- Optional script arguments pushed onto the stack.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a `ScriptError` if the trigger has no bound
    /// script or no subject was provided.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `accept_login` (the `Login` trigger), input handlers,
    /// player and NPC interactions, NPC timers and hunts, `close_modal` and
    /// `close_tutorial` (`IfClose`), VM ops (through `run_opheld`, and the
    /// harness's `test_op`).
    /// **Calls:** `script_by_key`, `run_script_inner`.
    pub fn run_script_by_trigger(
        &mut self,
        trigger: (ServerTriggerType, Option<i32>, Option<i32>),
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
        protect: Option<bool>,
        force: Option<bool>,
        args: Option<Vec<ScriptArgument>>,
    ) -> Result<(), ScriptError> {
        let Some(script) = self.script_by_key(trigger.0, trigger.1, trigger.2) else {
            return Err(ScriptError::TriggerNotFound(trigger.0));
        };
        self.run_script_inner(subject, target, protect, force, args, script)
            .unwrap_or_else(|value| value)
    }

    /// Looks up a script by its string name and executes it against a subject entity.
    ///
    /// This is the name-based counterpart to [`Engine::run_script_by_trigger`].
    /// Useful for explicitly invoking scripts by their defined name rather than
    /// by trigger binding (e.g. `[debugproc,...]` cheat commands, or a proc
    /// such as `[proc,clickworldmap]`).
    ///
    /// # Arguments
    ///
    /// * `name` -- The script name as defined in the script source (e.g. `"[proc,clickworldmap]"`).
    /// * `subject` -- The entity that owns the script execution.
    /// * `target` -- Optional secondary entity (e.g. interaction target).
    /// * `protect` -- Whether the owning player should be marked as protected.
    /// * `force` -- Whether to bypass protection/delay checks.
    /// * `args` -- Optional script arguments pushed onto the stack.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or a `ScriptError` if the script name is not found
    /// or no subject was provided.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `cheat_debugproc`, the `ClickWorldMap` and `WindowStatus`
    /// handlers, rs-test.
    /// **Calls:** `run_script_inner`.
    pub fn run_script_by_name(
        &mut self,
        name: &str,
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
        protect: Option<bool>,
        force: Option<bool>,
        args: Option<Vec<ScriptArgument>>,
    ) -> Result<(), ScriptError> {
        let Some(script) = self.scripts.get_by_name(name).cloned() else {
            return Err(ScriptError::ScriptNotFoundName(name.into()));
        };
        self.run_script_inner(subject, target, protect, force, args, script)
            .unwrap_or_else(|value| value)
    }

    /// Internal helper that builds a [`ScriptState`] and dispatches execution
    /// to the appropriate player or NPC handler. A player subject that a kill
    /// switch keeps from starting `script` is told so, and nothing runs.
    ///
    /// Used by both [`Engine::run_script_by_trigger`] and
    /// [`Engine::run_script_by_name`] to avoid code duplication.
    ///
    /// When `reusable_script` holds a state, it is taken and
    /// [`reset`](ScriptState::reset) is called to reinitialize it for the new
    /// script invocation, avoiding fresh heap allocations for the fixed-size
    /// stacks. After execution, if the script finished or aborted (i.e., was
    /// not suspended), the state is recaptured for future reuse. Suspended
    /// states are stored elsewhere and must not be reused until they complete.
    ///
    /// # Arguments
    ///
    /// * `subject` -- The entity that owns the script. `None` returns an error.
    /// * `target` -- Optional interaction target entity.
    /// * `protect` -- Whether to mark the player as protected.
    /// * `force` -- Whether to bypass protection/delay checks.
    /// * `args` -- Optional script arguments.
    /// * `script` -- The resolved script to execute (already cloned from the provider).
    ///
    /// # Returns
    ///
    /// `Ok(Ok(()))` on success. `Err(Err(ScriptError::NoSubject))` when no
    /// subject is provided. The nested `Result` allows the caller to use
    /// `unwrap_or_else` to flatten the error path.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `run_script_by_trigger`, `run_script_by_name`.
    /// **Calls:** `switched_off`, `ScriptState::reset` or `ScriptState::init`,
    /// `runescript_execute_script_player`, `runescript_execute_script_npc`.
    fn run_script_inner(
        &mut self,
        subject: Option<ScriptSubject>,
        target: Option<ScriptSubject>,
        protect: Option<bool>,
        force: Option<bool>,
        args: Option<Vec<ScriptArgument>>,
        script: Arc<Script>,
    ) -> Result<Result<(), ScriptError>, Result<(), ScriptError>> {
        let Some(subject) = subject else {
            return Err(Err(ScriptError::NoSubject));
        };

        // Determine which entity executor to use before moving subject
        // into the state. PlayerUid/NpcUid are Copy.
        enum SubjectKind {
            Player(PlayerUid),
            Npc(NpcUid),
            Other,
        }

        let kind = match &subject {
            ScriptSubject::Player(uid) => SubjectKind::Player(*uid),
            ScriptSubject::Npc(uid) => SubjectKind::Npc(*uid),
            ScriptSubject::Loc(_) | ScriptSubject::Obj(_) => SubjectKind::Other,
        };
        if let SubjectKind::Player(uid) = kind
            && self.switched_off(uid, &script)
        {
            return Ok(Ok(()));
        }

        // Take the reusable state if available, otherwise allocate fresh.
        let state = if let Some(mut reusable) = self.reusable_script.take() {
            reusable.reset(script, Some(subject), target, args);
            reusable
        } else {
            ScriptState::init(script, Some(subject), target, args)
        };

        // Execute and recapture the state if it was not suspended.
        let returned = match kind {
            SubjectKind::Player(uid) => {
                let protect = protect.unwrap_or(false);
                let force = force.unwrap_or(false);
                self.runescript_execute_script_player(uid, state, protect, force)
            }
            SubjectKind::Npc(uid) => self.runescript_execute_script_npc(uid, state),
            SubjectKind::Other => Some(state),
        };

        // Reclaim the state for reuse on the next invocation.
        if let Some(returned_state) = returned {
            self.reusable_script = Some(returned_state);
        }

        Ok(Ok(()))
    }

    /// Executes a RuneScript with a player as the subject, handling protection
    /// flags, suspension, and post-execution cleanup.
    ///
    /// If `protect` is true the player's `protect` flag is set before execution
    /// and cleared after. If `protect` is true, `force` is false and the
    /// player is already protected or delayed, execution is skipped entirely.
    ///
    /// After VM execution, the method inspects the returned [`ExecutionState`]:
    /// - `Finished` / `Aborted` -- clears the player's active script and
    ///   closes any open modal if appropriate.
    /// - `WorldSuspended` -- enqueues the script into the world queue with
    ///   the delay popped from the stack.
    /// - `NpcSuspended` -- parks the script on the active NPC's state.
    /// - Other suspension -- parks the script on the player's state, and the
    ///   player stays protected while it waits if `protect` is set.
    ///
    /// Also cleans up `ProtectedActivePlayer` / `ProtectedActivePlayer2`
    /// pointer flags to ensure no stale protection lingers.
    ///
    /// # Returns
    ///
    /// `Some(state)` when the script finished or aborted, or was skipped by
    /// the protection/delay guard (the caller may reclaim the state for
    /// reuse). `None` when the script was suspended and its state has been
    /// moved into storage for later resumption.
    ///
    /// # Arguments
    ///
    /// * `uid` -- The player's unique identifier (encodes pid).
    /// * `state` -- The script state to execute (consumed).
    /// * `protect` -- Whether to set the protection flag during execution.
    /// * `force` -- Whether to skip the protection/delay guard.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `run_script_by_state`, `run_script_inner`.
    /// **Calls:** `runescript_vm_execute`, `enqueue_world_script`.
    #[allow(clippy::collapsible_if)]
    fn runescript_execute_script_player(
        &mut self,
        uid: PlayerUid,
        mut state: ScriptState,
        protect: bool,
        force: bool,
    ) -> Option<ScriptState> {
        let pid = uid.pid() as usize;
        if !force && protect {
            if let Some(active) = &mut self.player_list.players[pid] {
                if active.player.state.protect || active.player.state.delayed {
                    return Some(state);
                }
            }
        }

        if protect {
            if let Some(active) = &mut self.player_list.players[pid] {
                state.pointers.add(ScriptPointer::ProtectedActivePlayer);
                active.player.state.protect = true;
            }
        }

        let result = self.runescript_vm_execute(&mut state);

        if protect {
            if let Some(active) = &mut self.player_list.players[pid] {
                active.player.state.protect = false;
            }
        }

        self.release_protected_players(&mut state);

        if result != ExecutionState::Finished && result != ExecutionState::Aborted {
            if result == ExecutionState::WorldSuspended {
                let delay = state.pop_int();
                self.enqueue_world_script(state, delay);
            } else if result == ExecutionState::NpcSuspended {
                let npc_uid = if state.int_operand() == 0 {
                    state.active_npc
                } else {
                    state.active_npc2
                };
                if let Some(uid) = npc_uid {
                    if let Some(active) = self.npc_list.get_mut(uid.nid()) {
                        active.npc.state.active_script = Some(Box::new(state));
                    }
                }
            } else if let Some(active) = &mut self.player_list.players[pid] {
                active.player.state.active_script = Some(Box::new(state));
                active.player.state.protect = protect;
            }
            None
        } else {
            if let Some(active) = &mut self.player_list.players[pid] {
                if let Some(active_script) = &active.player.state.active_script {
                    if active_script.root_script_id == state.root_script_id {
                        active.player.state.active_script = None;
                        // A dialogue that ends on a choice leaves its resume buttons: with no
                        // suspended script left, close_modal does not clear them, and 647's
                        // RESUME_PAUSEBUTTON check would refuse the next dialogue's button.
                        #[cfg(since_647)]
                        {
                            active.player.resume_buttons = None;
                        }
                        if active.player.modal_state & MODAL_MAIN == MODAL_NONE {
                            if let Err(e) = active.close_modal(false) {
                                error!(
                                    "error closing modal after script finish for player {pid}: {e}"
                                );
                            }
                        }
                    }
                }
            }
            Some(state)
        }
    }

    /// Executes a RuneScript with an NPC as the subject, handling suspension
    /// and post-execution cleanup.
    ///
    /// Similar to [`Engine::runescript_execute_script_player`] but without
    /// protection/force semantics (NPCs are never "protected"). After
    /// execution the method routes suspended scripts to the appropriate
    /// entity (world queue, NPC state, or a referenced player).
    ///
    /// Also cleans up `ProtectedActivePlayer` / `ProtectedActivePlayer2`
    /// pointer flags on any players that were marked during script execution.
    ///
    /// # Returns
    ///
    /// `Some(state)` when the script finished or aborted (the caller may
    /// reclaim the state for reuse). `None` when the script was suspended
    /// and its state has been moved into storage for later resumption.
    ///
    /// # Arguments
    ///
    /// * `uid` -- The NPC's unique identifier (encodes nid).
    /// * `state` -- The script state to execute (consumed).
    ///
    /// # Call Stack
    ///
    /// **Called by:** `run_script_by_state`, `run_script_inner`.
    /// **Calls:** `runescript_vm_execute`, `enqueue_world_script`.
    #[allow(clippy::collapsible_if)]
    fn runescript_execute_script_npc(
        &mut self,
        uid: NpcUid,
        mut state: ScriptState,
    ) -> Option<ScriptState> {
        let result = self.runescript_vm_execute(&mut state);

        self.release_protected_players(&mut state);

        if result != ExecutionState::Finished && result != ExecutionState::Aborted {
            if result == ExecutionState::WorldSuspended {
                let delay = state.pop_int();
                self.enqueue_world_script(state, delay);
            } else if result == ExecutionState::NpcSuspended {
                if let Some(active) = &mut self.npc_list.npcs[uid.nid() as usize] {
                    active.npc.state.active_script = Some(Box::new(state));
                }
            } else if let Some(uid) = state.active_player {
                if let Some(active) = &mut self.player_list.players[uid.pid() as usize] {
                    active.player.state.active_script = Some(Box::new(state));
                }
            } else if let Some(uid) = state.active_player2 {
                if let Some(active) = &mut self.player_list.players[uid.pid() as usize] {
                    active.player.state.active_script = Some(Box::new(state));
                }
            }
            None
        } else {
            if let Some(active) = &mut self.npc_list.npcs[uid.nid() as usize] {
                if let Some(active_script) = &active.npc.state.active_script {
                    if active_script.root_script_id == state.root_script_id {
                        active.npc.state.active_script = None;
                    }
                }
            }
            Some(state)
        }
    }

    /// Drops the protected access a script holds on its active players.
    ///
    /// For each `ProtectedActivePlayer` / `ProtectedActivePlayer2` pointer
    /// with a bound player, clears that player's `protect` flag and removes
    /// the pointer. Runs after every VM execution so a script that suspends
    /// (`p_delay`, `npc_delay`, `world_delay`) must `p_finduid` again before
    /// it can touch protected state on resume.
    ///
    /// # Arguments
    ///
    /// * `state` -- The script state that just returned from the VM.
    ///
    /// # Call Stack
    ///
    /// **Called by:** `runescript_execute_script_player`,
    /// `runescript_execute_script_npc`, `run_world_script`.
    pub(crate) fn release_protected_players(&mut self, state: &mut ScriptState) {
        let players = [state.active_player, state.active_player2];
        for (pointer, uid) in ScriptState::PROTECTED_ACTIVE_PLAYER
            .into_iter()
            .zip(players)
        {
            let Some(uid) = uid else {
                continue;
            };
            if !state.pointers.has(pointer) {
                continue;
            }
            if let Some(active) = &mut self.player_list.players[uid.pid() as usize] {
                active.player.state.protect = false;
            }
            state.pointers.remove(pointer);
        }
    }

    /// Enqueues a suspended script into the world script queue with a tick delay.
    ///
    /// World scripts are not bound to a specific player or NPC -- they execute
    /// in the world phase and can interact with any entity. The delay is
    /// incremented by 1 internally (so `delay=0` fires on the next tick).
    /// Negative delays are kept as they are, like the reference engine's
    /// `enqueueScript`; the world queue runs them on its next pass.
    ///
    /// # Arguments
    ///
    /// * `script` -- The suspended script state to enqueue.
    /// * `delay` -- Number of additional ticks before the script resumes.
    ///
    /// # Side Effects
    ///
    /// Appends the script to `self.world_queue`.
    pub fn enqueue_world_script(&mut self, mut script: ScriptState, delay: i32) {
        script.delay = delay.saturating_add(1);
        self.world_queue.add_tail(script);
    }
}
